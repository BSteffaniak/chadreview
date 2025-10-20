use crate::diff_parser::parse_unified_diff;
use anyhow::Result;
use chadreview_git_provider::GitProvider;
use chadreview_pr_models::{
    Comment, CreateComment, DiffFile, FileStatus, Label, PrState, PullRequest, User,
};
use chadreview_syntax::SyntaxHighlighter;

pub struct GitHubProvider {
    http_client: reqwest::Client,
    auth_token: Option<String>,
    base_url: String,
}

impl GitHubProvider {
    /// Create a new GitHub provider without authentication.
    ///
    /// # Panics
    ///
    /// * If the `reqwest::Client` fails to build.
    #[must_use]
    pub fn new() -> Self {
        let http_client = reqwest::Client::builder()
            .user_agent("ChadReview")
            .build()
            .unwrap();
        Self {
            http_client,
            auth_token: None,
            base_url: "https://api.github.com".to_string(),
        }
    }

    #[must_use]
    pub fn with_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

impl Default for GitHubProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl GitProvider for GitHubProvider {
    async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );
        log::debug!("GET {url}");
        let mut request = self
            .http_client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        let pr_data: serde_json::Value = response.json().await?;

        Ok(PullRequest {
            number: pr_data["number"].as_u64().unwrap(),
            owner: owner.to_string(),
            repo: repo.to_string(),
            title: pr_data["title"].as_str().unwrap().to_string(),
            description: pr_data["body"].as_str().unwrap_or("").to_string(),
            author: parse_user(&pr_data["user"]),
            state: parse_pr_state(&pr_data),
            draft: pr_data["draft"].as_bool().unwrap_or(false),
            base_branch: pr_data["base"]["ref"].as_str().unwrap().to_string(),
            head_branch: pr_data["head"]["ref"].as_str().unwrap().to_string(),
            labels: parse_labels(&pr_data["labels"]),
            assignees: parse_users(&pr_data["assignees"]),
            reviewers: parse_users(&pr_data["requested_reviewers"]),
            commits: vec![],
            created_at: parse_datetime(pr_data["created_at"].as_str().unwrap()),
            updated_at: parse_datetime(pr_data["updated_at"].as_str().unwrap()),
            provider: "github".to_string(),
        })
    }

    async fn get_diff(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<DiffFile>> {
        let files_url = format!(
            "{}/repos/{}/{}/pulls/{}/files",
            self.base_url, owner, repo, number
        );
        log::debug!("GET {files_url}");
        let mut files_request = self
            .http_client
            .get(&files_url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.auth_token {
            files_request = files_request.bearer_auth(token);
        }

        let files_response = files_request.send().await?;
        let status = files_response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", files_response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        let files_data: Vec<serde_json::Value> = files_response.json().await?;

        let diff_url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );
        let mut diff_request = self
            .http_client
            .get(&diff_url)
            .header("Accept", "application/vnd.github.v3.diff");

        if let Some(token) = &self.auth_token {
            diff_request = diff_request.bearer_auth(token);
        }

        let diff_response = diff_request.send().await?;
        let status = diff_response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", diff_response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        let full_diff = diff_response.text().await?;

        let highlighter = SyntaxHighlighter::new();
        let mut result = Vec::new();

        for file_data in &files_data {
            let filename = file_data["filename"].as_str().unwrap();
            let status = parse_file_status(file_data["status"].as_str().unwrap());
            let additions = usize::try_from(file_data["additions"].as_u64().unwrap())?;
            let deletions = usize::try_from(file_data["deletions"].as_u64().unwrap())?;

            let file_diff = extract_file_diff(&full_diff, filename);

            if let Some(diff_text) = file_diff {
                let parsed = parse_unified_diff(
                    filename,
                    status,
                    additions,
                    deletions,
                    &diff_text,
                    &highlighter,
                )
                .map_err(|e| anyhow::anyhow!(e))?;
                result.push(parsed);
            }
        }

        Ok(result)
    }

    async fn get_comments(&self, owner: &str, repo: &str, number: u64) -> Result<Vec<Comment>> {
        let review_comments_url = format!(
            "{}/repos/{}/{}/pulls/{}/comments",
            self.base_url, owner, repo, number
        );
        log::debug!("GET {review_comments_url}");
        let mut review_request = self
            .http_client
            .get(&review_comments_url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.auth_token {
            review_request = review_request.bearer_auth(token);
        }

        let review_response = review_request.send().await?;
        let status = review_response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", review_response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        let review_comments: Vec<serde_json::Value> = review_response.json().await?;

        let issue_comments_url = format!(
            "{}/repos/{}/{}/issues/{}/comments",
            self.base_url, owner, repo, number
        );
        let mut issue_request = self
            .http_client
            .get(&issue_comments_url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.auth_token {
            issue_request = issue_request.bearer_auth(token);
        }

        let issue_response = issue_request.send().await?;
        let status = issue_response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", issue_response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        let issue_comments: Vec<serde_json::Value> = issue_response.json().await?;

        let mut all_comments_with_reply_info = Vec::new();

        for comment_data in &review_comments {
            let comment = parse_review_comment(comment_data);
            let in_reply_to = comment_data["in_reply_to_id"].as_u64();
            all_comments_with_reply_info.push((comment, in_reply_to));
        }

        for comment_data in &issue_comments {
            let comment = parse_issue_comment(comment_data);
            all_comments_with_reply_info.push((comment, None));
        }

        Ok(thread_comments(all_comments_with_reply_info))
    }

    async fn create_comment(
        &self,
        owner: &str,
        repo: &str,
        number: u64,
        comment: CreateComment,
    ) -> Result<Comment> {
        use chadreview_pr_models::CommentType;

        match &comment.comment_type {
            CommentType::LineLevelComment { path, line } => {
                let url = format!(
                    "{}/repos/{}/{}/pulls/{}/comments",
                    self.base_url, owner, repo, number
                );
                log::debug!("POST {url}");

                let mut body = serde_json::json!({
                    "body": comment.body,
                    "path": path,
                    "line": line,
                });

                if let Some(reply_to) = comment.in_reply_to {
                    body["in_reply_to"] = serde_json::json!(reply_to);
                }

                let mut request = self
                    .http_client
                    .post(&url)
                    .header("Accept", "application/vnd.github.v3+json")
                    .json(&body);

                if let Some(token) = &self.auth_token {
                    request = request.bearer_auth(token);
                }

                let response = request.send().await?;
                let status = response.status();

                if !status.is_success() {
                    log::error!("GitHub API error: {}", response.text().await?);
                    anyhow::bail!("GitHub API error: {status}");
                }

                let comment_data: serde_json::Value = response.json().await?;
                Ok(parse_review_comment(&comment_data))
            }
            CommentType::FileLevelComment { path } => {
                let url = format!(
                    "{}/repos/{}/{}/pulls/{}/comments",
                    self.base_url, owner, repo, number
                );
                log::debug!("POST {url}");

                let mut body = serde_json::json!({
                    "body": comment.body,
                    "path": path,
                });

                if let Some(reply_to) = comment.in_reply_to {
                    body["in_reply_to"] = serde_json::json!(reply_to);
                }

                let mut request = self
                    .http_client
                    .post(&url)
                    .header("Accept", "application/vnd.github.v3+json")
                    .json(&body);

                if let Some(token) = &self.auth_token {
                    request = request.bearer_auth(token);
                }

                let response = request.send().await?;
                let status = response.status();

                if !status.is_success() {
                    log::error!("GitHub API error: {}", response.text().await?);
                    anyhow::bail!("GitHub API error: {status}");
                }

                let comment_data: serde_json::Value = response.json().await?;
                Ok(parse_review_comment(&comment_data))
            }
            CommentType::General => {
                let url = format!(
                    "{}/repos/{}/{}/issues/{}/comments",
                    self.base_url, owner, repo, number
                );
                log::debug!("POST {url}");

                let body = serde_json::json!({
                    "body": comment.body,
                });

                let mut request = self
                    .http_client
                    .post(&url)
                    .header("Accept", "application/vnd.github.v3+json")
                    .json(&body);

                if let Some(token) = &self.auth_token {
                    request = request.bearer_auth(token);
                }

                let response = request.send().await?;
                let status = response.status();

                if !status.is_success() {
                    log::error!("GitHub API error: {}", response.text().await?);
                    anyhow::bail!("GitHub API error: {status}");
                }

                let comment_data: serde_json::Value = response.json().await?;
                Ok(parse_issue_comment(&comment_data))
            }
        }
    }

    async fn update_comment(&self, comment_id: u64, body: String) -> Result<Comment> {
        let request_body = serde_json::json!({
            "body": body,
        });

        let review_url = format!("{}/repos/*/pulls/comments/{}", self.base_url, comment_id);
        log::debug!("PATCH {review_url}");
        let mut review_request = self
            .http_client
            .patch(&review_url)
            .header("Accept", "application/vnd.github.v3+json")
            .json(&request_body);

        if let Some(token) = &self.auth_token {
            review_request = review_request.bearer_auth(token);
        }

        let review_response = review_request.send().await?;

        if review_response.status().is_success() {
            let comment_data: serde_json::Value = review_response.json().await?;
            return Ok(parse_review_comment(&comment_data));
        }

        let issue_url = format!("{}/repos/*/issues/comments/{}", self.base_url, comment_id);
        log::debug!("PATCH {issue_url}");
        let mut issue_request = self
            .http_client
            .patch(&issue_url)
            .header("Accept", "application/vnd.github.v3+json")
            .json(&request_body);

        if let Some(token) = &self.auth_token {
            issue_request = issue_request.bearer_auth(token);
        }

        let issue_response = issue_request.send().await?;
        let status = issue_response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", issue_response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        let comment_data: serde_json::Value = issue_response.json().await?;
        Ok(parse_issue_comment(&comment_data))
    }

    async fn delete_comment(&self, comment_id: u64) -> Result<()> {
        let review_url = format!("{}/repos/*/pulls/comments/{}", self.base_url, comment_id);
        log::debug!("DELETE {review_url}");
        let mut review_request = self
            .http_client
            .delete(&review_url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.auth_token {
            review_request = review_request.bearer_auth(token);
        }

        let review_response = review_request.send().await?;

        if review_response.status().is_success() {
            return Ok(());
        }

        let issue_url = format!("{}/repos/*/issues/comments/{}", self.base_url, comment_id);
        log::debug!("DELETE {issue_url}");
        let mut issue_request = self
            .http_client
            .delete(&issue_url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = &self.auth_token {
            issue_request = issue_request.bearer_auth(token);
        }

        let issue_response = issue_request.send().await?;
        let status = issue_response.status();

        if !status.is_success() {
            log::error!("GitHub API error: {}", issue_response.text().await?);
            anyhow::bail!("GitHub API error: {status}");
        }

        Ok(())
    }

    fn provider_name(&self) -> &'static str {
        "github"
    }

    fn supports_drafts(&self) -> bool {
        true
    }
}

fn parse_user(value: &serde_json::Value) -> User {
    User {
        id: value["id"].as_u64().unwrap().to_string(),
        username: value["login"].as_str().unwrap().to_string(),
        avatar_url: value["avatar_url"].as_str().unwrap().to_string(),
        html_url: value["html_url"].as_str().unwrap().to_string(),
    }
}

fn parse_users(value: &serde_json::Value) -> Vec<User> {
    value
        .as_array()
        .map(|arr| arr.iter().map(parse_user).collect())
        .unwrap_or_default()
}

fn parse_pr_state(value: &serde_json::Value) -> PrState {
    match value["state"].as_str().unwrap() {
        "closed" if value["merged"].as_bool().unwrap_or(false) => PrState::Merged,
        "closed" => PrState::Closed,
        _ => PrState::Open,
    }
}

fn parse_labels(value: &serde_json::Value) -> Vec<Label> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .map(|v| Label {
                    name: v["name"].as_str().unwrap().to_string(),
                    color: v["color"].as_str().unwrap().to_string(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_datetime(s: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(s)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

fn parse_file_status(status: &str) -> FileStatus {
    match status {
        "added" => FileStatus::Added,
        "removed" => FileStatus::Deleted,
        "renamed" => FileStatus::Renamed,
        _ => FileStatus::Modified,
    }
}

fn extract_file_diff(full_diff: &str, filename: &str) -> Option<String> {
    let file_marker = format!("diff --git a/{filename} b/{filename}");
    let start = full_diff.find(&file_marker)?;

    let rest = &full_diff[start..];
    let next_file = rest[1..].find("diff --git ");

    let end = next_file.map_or(rest.len(), |pos| pos + 1);
    Some(rest[..end].to_string())
}

fn parse_review_comment(value: &serde_json::Value) -> Comment {
    use chadreview_pr_models::CommentType;

    let path = value["path"].as_str().unwrap_or("").to_string();
    let line = value["line"].as_u64().and_then(|l| usize::try_from(l).ok());

    let comment_type = if let Some(line_num) = line {
        CommentType::LineLevelComment {
            path,
            line: line_num,
        }
    } else {
        CommentType::FileLevelComment { path }
    };

    Comment {
        id: value["id"].as_u64().unwrap(),
        author: parse_user(&value["user"]),
        body: value["body"].as_str().unwrap().to_string(),
        created_at: parse_datetime(value["created_at"].as_str().unwrap()),
        updated_at: parse_datetime(value["updated_at"].as_str().unwrap()),
        comment_type,
        replies: Vec::new(),
    }
}

fn parse_issue_comment(value: &serde_json::Value) -> Comment {
    use chadreview_pr_models::CommentType;

    Comment {
        id: value["id"].as_u64().unwrap(),
        author: parse_user(&value["user"]),
        body: value["body"].as_str().unwrap().to_string(),
        created_at: parse_datetime(value["created_at"].as_str().unwrap()),
        updated_at: parse_datetime(value["updated_at"].as_str().unwrap()),
        comment_type: CommentType::General,
        replies: Vec::new(),
    }
}

fn build_tree(
    comment_id: u64,
    comment_map: &mut std::collections::HashMap<u64, Comment>,
    reply_map: &std::collections::HashMap<u64, Vec<u64>>,
) -> Option<Comment> {
    let mut comment = comment_map.remove(&comment_id)?;

    if let Some(reply_ids) = reply_map.get(&comment_id) {
        for &reply_id in reply_ids {
            if let Some(reply) = build_tree(reply_id, comment_map, reply_map) {
                comment.replies.push(reply);
            }
        }
    }

    Some(comment)
}

fn thread_comments(comments_with_replies: Vec<(Comment, Option<u64>)>) -> Vec<Comment> {
    use std::collections::HashMap;

    let mut comment_map: HashMap<u64, Comment> = HashMap::new();
    let mut reply_map: HashMap<u64, Vec<u64>> = HashMap::new();
    let mut root_ids = Vec::new();

    for (comment, in_reply_to) in comments_with_replies {
        let comment_id = comment.id;
        comment_map.insert(comment_id, comment);

        if let Some(parent_id) = in_reply_to {
            reply_map.entry(parent_id).or_default().push(comment_id);
        } else {
            root_ids.push(comment_id);
        }
    }

    let mut result = Vec::new();
    for root_id in root_ids {
        if let Some(comment) = build_tree(root_id, &mut comment_map, &reply_map) {
            result.push(comment);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_get_pr_success() {
        let mock_server = MockServer::start().await;

        let pr_json = serde_json::json!({
            "number": 123,
            "title": "Test PR",
            "body": "Test description",
            "state": "open",
            "draft": false,
            "user": {
                "id": 12345,
                "login": "testuser",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/testuser"
            },
            "base": { "ref": "main" },
            "head": { "ref": "feature-branch" },
            "labels": [],
            "assignees": [],
            "requested_reviewers": [],
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-02T00:00:00Z",
            "merged": false
        });

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&pr_json))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let pr = client.get_pr("owner", "repo", 123).await.unwrap();

        assert_eq!(pr.number, 123);
        assert_eq!(pr.title, "Test PR");
        assert_eq!(pr.state, PrState::Open);
        assert_eq!(pr.author.username, "testuser");
        assert_eq!(pr.provider, "github");
    }

    #[tokio::test]
    async fn test_get_pr_merged_state() {
        let mock_server = MockServer::start().await;

        let pr_json = serde_json::json!({
            "number": 456,
            "title": "Merged PR",
            "body": "",
            "state": "closed",
            "merged": true,
            "draft": false,
            "user": {
                "id": 67890,
                "login": "author",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/author"
            },
            "base": { "ref": "main" },
            "head": { "ref": "feature" },
            "labels": [],
            "assignees": [],
            "requested_reviewers": [],
            "created_at": "2025-01-01T00:00:00Z",
            "updated_at": "2025-01-02T00:00:00Z"
        });

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/456"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&pr_json))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let pr = client.get_pr("owner", "repo", 456).await.unwrap();

        assert_eq!(pr.state, PrState::Merged);
    }

    #[tokio::test]
    async fn test_get_comments_general() {
        let mock_server = MockServer::start().await;

        let review_comments = serde_json::json!([]);
        let issue_comments = serde_json::json!([
            {
                "id": 1001,
                "body": "This looks great!",
                "user": {
                    "id": 12345,
                    "login": "reviewer1",
                    "avatar_url": "https://example.com/avatar1.png",
                    "html_url": "https://github.com/reviewer1"
                },
                "created_at": "2025-01-01T10:00:00Z",
                "updated_at": "2025-01-01T10:00:00Z"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&review_comments))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&issue_comments))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let comments = client.get_comments("owner", "repo", 123).await.unwrap();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, 1001);
        assert_eq!(comments[0].body, "This looks great!");
        assert_eq!(comments[0].author.username, "reviewer1");
        assert!(matches!(
            comments[0].comment_type,
            chadreview_pr_models::CommentType::General
        ));
        assert_eq!(comments[0].replies.len(), 0);
    }

    #[tokio::test]
    async fn test_get_comments_line_level() {
        let mock_server = MockServer::start().await;

        let review_comments = serde_json::json!([
            {
                "id": 2001,
                "body": "This needs fixing",
                "path": "src/main.rs",
                "line": 42,
                "user": {
                    "id": 12345,
                    "login": "reviewer1",
                    "avatar_url": "https://example.com/avatar1.png",
                    "html_url": "https://github.com/reviewer1"
                },
                "created_at": "2025-01-01T10:00:00Z",
                "updated_at": "2025-01-01T10:00:00Z",
                "in_reply_to_id": null
            }
        ]);
        let issue_comments = serde_json::json!([]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&review_comments))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&issue_comments))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let comments = client.get_comments("owner", "repo", 123).await.unwrap();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, 2001);
        assert_eq!(comments[0].body, "This needs fixing");
        match &comments[0].comment_type {
            chadreview_pr_models::CommentType::LineLevelComment { path, line } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(*line, 42);
            }
            _ => panic!("Expected LineLevelComment"),
        }
    }

    #[tokio::test]
    async fn test_get_comments_threaded() {
        let mock_server = MockServer::start().await;

        let review_comments = serde_json::json!([
            {
                "id": 3001,
                "body": "Parent comment",
                "path": "src/lib.rs",
                "line": 10,
                "user": {
                    "id": 12345,
                    "login": "reviewer1",
                    "avatar_url": "https://example.com/avatar1.png",
                    "html_url": "https://github.com/reviewer1"
                },
                "created_at": "2025-01-01T10:00:00Z",
                "updated_at": "2025-01-01T10:00:00Z",
                "in_reply_to_id": null
            },
            {
                "id": 3002,
                "body": "Reply to parent",
                "path": "src/lib.rs",
                "line": 10,
                "user": {
                    "id": 67890,
                    "login": "author",
                    "avatar_url": "https://example.com/avatar2.png",
                    "html_url": "https://github.com/author"
                },
                "created_at": "2025-01-01T11:00:00Z",
                "updated_at": "2025-01-01T11:00:00Z",
                "in_reply_to_id": 3001
            }
        ]);
        let issue_comments = serde_json::json!([]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&review_comments))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&issue_comments))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let comments = client.get_comments("owner", "repo", 123).await.unwrap();

        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].id, 3001);
        assert_eq!(comments[0].body, "Parent comment");
        assert_eq!(comments[0].replies.len(), 1);
        assert_eq!(comments[0].replies[0].id, 3002);
        assert_eq!(comments[0].replies[0].body, "Reply to parent");
        assert_eq!(comments[0].replies[0].author.username, "author");
    }

    #[tokio::test]
    async fn test_get_comments_mixed_types() {
        let mock_server = MockServer::start().await;

        let review_comments = serde_json::json!([
            {
                "id": 4001,
                "body": "Line comment",
                "path": "src/main.rs",
                "line": 5,
                "user": {
                    "id": 12345,
                    "login": "reviewer1",
                    "avatar_url": "https://example.com/avatar1.png",
                    "html_url": "https://github.com/reviewer1"
                },
                "created_at": "2025-01-01T10:00:00Z",
                "updated_at": "2025-01-01T10:00:00Z",
                "in_reply_to_id": null
            }
        ]);
        let issue_comments = serde_json::json!([
            {
                "id": 4002,
                "body": "General PR comment",
                "user": {
                    "id": 67890,
                    "login": "author",
                    "avatar_url": "https://example.com/avatar2.png",
                    "html_url": "https://github.com/author"
                },
                "created_at": "2025-01-01T11:00:00Z",
                "updated_at": "2025-01-01T11:00:00Z"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&review_comments))
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/issues/123/comments"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&issue_comments))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let comments = client.get_comments("owner", "repo", 123).await.unwrap();

        assert_eq!(comments.len(), 2);

        let line_comment = comments.iter().find(|c| c.id == 4001).unwrap();
        assert!(matches!(
            line_comment.comment_type,
            chadreview_pr_models::CommentType::LineLevelComment { .. }
        ));

        let general_comment = comments.iter().find(|c| c.id == 4002).unwrap();
        assert!(matches!(
            general_comment.comment_type,
            chadreview_pr_models::CommentType::General
        ));
    }

    #[tokio::test]
    async fn test_create_comment_line_level() {
        let mock_server = MockServer::start().await;

        let comment_response = serde_json::json!({
            "id": 5001,
            "body": "New line comment",
            "path": "src/main.rs",
            "line": 10,
            "user": {
                "id": 12345,
                "login": "commenter",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/commenter"
            },
            "created_at": "2025-01-01T12:00:00Z",
            "updated_at": "2025-01-01T12:00:00Z",
            "in_reply_to_id": null
        });

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&comment_response))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let create_comment = CreateComment {
            body: "New line comment".to_string(),
            comment_type: chadreview_pr_models::CommentType::LineLevelComment {
                path: "src/main.rs".to_string(),
                line: 10,
            },
            in_reply_to: None,
        };

        let comment = client
            .create_comment("owner", "repo", 123, create_comment)
            .await
            .unwrap();

        assert_eq!(comment.id, 5001);
        assert_eq!(comment.body, "New line comment");
        assert_eq!(comment.author.username, "commenter");
        match &comment.comment_type {
            chadreview_pr_models::CommentType::LineLevelComment { path, line } => {
                assert_eq!(path, "src/main.rs");
                assert_eq!(*line, 10);
            }
            _ => panic!("Expected LineLevelComment"),
        }
    }

    #[tokio::test]
    async fn test_create_comment_general() {
        let mock_server = MockServer::start().await;

        let comment_response = serde_json::json!({
            "id": 5002,
            "body": "New general comment",
            "user": {
                "id": 12345,
                "login": "commenter",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/commenter"
            },
            "created_at": "2025-01-01T12:00:00Z",
            "updated_at": "2025-01-01T12:00:00Z"
        });

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/issues/123/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&comment_response))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let create_comment = CreateComment {
            body: "New general comment".to_string(),
            comment_type: chadreview_pr_models::CommentType::General,
            in_reply_to: None,
        };

        let comment = client
            .create_comment("owner", "repo", 123, create_comment)
            .await
            .unwrap();

        assert_eq!(comment.id, 5002);
        assert_eq!(comment.body, "New general comment");
        assert!(matches!(
            comment.comment_type,
            chadreview_pr_models::CommentType::General
        ));
    }

    #[tokio::test]
    async fn test_create_comment_reply() {
        let mock_server = MockServer::start().await;

        let comment_response = serde_json::json!({
            "id": 5003,
            "body": "Reply to comment",
            "path": "src/lib.rs",
            "line": 5,
            "user": {
                "id": 12345,
                "login": "commenter",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/commenter"
            },
            "created_at": "2025-01-01T12:00:00Z",
            "updated_at": "2025-01-01T12:00:00Z",
            "in_reply_to_id": 3001
        });

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(201).set_body_json(&comment_response))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let create_comment = CreateComment {
            body: "Reply to comment".to_string(),
            comment_type: chadreview_pr_models::CommentType::LineLevelComment {
                path: "src/lib.rs".to_string(),
                line: 5,
            },
            in_reply_to: Some(3001),
        };

        let comment = client
            .create_comment("owner", "repo", 123, create_comment)
            .await
            .unwrap();

        assert_eq!(comment.id, 5003);
        assert_eq!(comment.body, "Reply to comment");
    }

    #[tokio::test]
    async fn test_create_comment_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/repos/owner/repo/pulls/123/comments"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("bad-token".to_string())
            .with_base_url(mock_server.uri());

        let create_comment = CreateComment {
            body: "Should fail".to_string(),
            comment_type: chadreview_pr_models::CommentType::LineLevelComment {
                path: "src/main.rs".to_string(),
                line: 1,
            },
            in_reply_to: None,
        };

        let result = client
            .create_comment("owner", "repo", 123, create_comment)
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_update_comment_review() {
        let mock_server = MockServer::start().await;

        let updated_comment = serde_json::json!({
            "id": 6001,
            "body": "Updated comment body",
            "path": "src/main.rs",
            "line": 10,
            "user": {
                "id": 12345,
                "login": "commenter",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/commenter"
            },
            "created_at": "2025-01-01T12:00:00Z",
            "updated_at": "2025-01-01T13:00:00Z",
            "in_reply_to_id": null
        });

        Mock::given(method("PATCH"))
            .and(path("/repos/*/pulls/comments/6001"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&updated_comment))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let comment = client
            .update_comment(6001, "Updated comment body".to_string())
            .await
            .unwrap();

        assert_eq!(comment.id, 6001);
        assert_eq!(comment.body, "Updated comment body");
    }

    #[tokio::test]
    async fn test_update_comment_issue() {
        let mock_server = MockServer::start().await;

        let updated_comment = serde_json::json!({
            "id": 6002,
            "body": "Updated general comment",
            "user": {
                "id": 12345,
                "login": "commenter",
                "avatar_url": "https://example.com/avatar.png",
                "html_url": "https://github.com/commenter"
            },
            "created_at": "2025-01-01T12:00:00Z",
            "updated_at": "2025-01-01T13:00:00Z"
        });

        Mock::given(method("PATCH"))
            .and(path("/repos/*/pulls/comments/6002"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("PATCH"))
            .and(path("/repos/*/issues/comments/6002"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&updated_comment))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let comment = client
            .update_comment(6002, "Updated general comment".to_string())
            .await
            .unwrap();

        assert_eq!(comment.id, 6002);
        assert_eq!(comment.body, "Updated general comment");
        assert!(matches!(
            comment.comment_type,
            chadreview_pr_models::CommentType::General
        ));
    }

    #[tokio::test]
    async fn test_update_comment_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PATCH"))
            .and(path("/repos/*/pulls/comments/6003"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        Mock::given(method("PATCH"))
            .and(path("/repos/*/issues/comments/6003"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("bad-token".to_string())
            .with_base_url(mock_server.uri());

        let result = client.update_comment(6003, "Should fail".to_string()).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_comment_review() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/repos/*/pulls/comments/7001"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let result = client.delete_comment(7001).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_comment_issue() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/repos/*/pulls/comments/7002"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock_server)
            .await;

        Mock::given(method("DELETE"))
            .and(path("/repos/*/issues/comments/7002"))
            .respond_with(ResponseTemplate::new(204))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let result = client.delete_comment(7002).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_delete_comment_unauthorized() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/repos/*/pulls/comments/7003"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock_server)
            .await;

        Mock::given(method("DELETE"))
            .and(path("/repos/*/issues/comments/7003"))
            .respond_with(ResponseTemplate::new(403))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("bad-token".to_string())
            .with_base_url(mock_server.uri());

        let result = client.delete_comment(7003).await;

        assert!(result.is_err());
    }
}
