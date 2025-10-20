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
        let files_data = fetch_all_pr_files(
            &self.http_client,
            &self.base_url,
            owner,
            repo,
            number,
            self.auth_token.as_ref(),
        )
        .await?;

        let highlighter = SyntaxHighlighter::new();
        let mut result = Vec::new();

        for file_data in &files_data {
            let filename = file_data["filename"].as_str().unwrap();
            let status = parse_file_status(file_data["status"].as_str().unwrap());
            let additions = usize::try_from(file_data["additions"].as_u64().unwrap())?;
            let deletions = usize::try_from(file_data["deletions"].as_u64().unwrap())?;

            if let Some(patch_str) = file_data["patch"].as_str() {
                let parsed = parse_unified_diff(
                    filename,
                    status,
                    additions,
                    deletions,
                    patch_str,
                    &highlighter,
                )
                .map_err(|e| anyhow::anyhow!(e))?;
                result.push(parsed);
            } else {
                log::debug!(
                    "Skipping {filename} - no patch data (likely binary or no content changes)"
                );
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

#[derive(Debug, Default)]
struct LinkHeader {
    next: Option<String>,
}

fn parse_link_header(header_value: &str) -> LinkHeader {
    let mut result = LinkHeader::default();

    for part in header_value.split(',') {
        let segments: Vec<&str> = part.split(';').collect();
        if segments.len() != 2 {
            continue;
        }

        let url = segments[0]
            .trim()
            .trim_start_matches('<')
            .trim_end_matches('>');
        let rel = segments[1].trim();

        if let Some(rel_value) = rel.strip_prefix("rel=\"").and_then(|r| r.strip_suffix('"'))
            && rel_value == "next"
        {
            result.next = Some(url.to_string());
        }
    }

    result
}

async fn fetch_all_pr_files(
    http_client: &reqwest::Client,
    base_url: &str,
    owner: &str,
    repo: &str,
    number: u64,
    auth_token: Option<&String>,
) -> Result<Vec<serde_json::Value>> {
    const MAX_FILES: usize = 3000;
    const PER_PAGE: u32 = 100;

    let mut all_files = Vec::new();
    let mut page = 1;

    log::debug!("Fetching PR files for {owner}/{repo} #{number}");

    loop {
        let url = format!(
            "{base_url}/repos/{owner}/{repo}/pulls/{number}/files?per_page={PER_PAGE}&page={page}"
        );

        log::debug!("GET {url} (page {page})");

        let mut request = http_client
            .get(&url)
            .header("Accept", "application/vnd.github.v3+json");

        if let Some(token) = auth_token {
            request = request.bearer_auth(token);
        }

        let response = request.send().await?;
        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await?;
            log::error!("GitHub API error on page {page}: {error_text}");
            anyhow::bail!("GitHub API error: {status}");
        }

        let link_header = response
            .headers()
            .get("Link")
            .and_then(|h| h.to_str().ok())
            .map(parse_link_header);

        let page_files: Vec<serde_json::Value> = response.json().await?;
        let files_in_page = page_files.len();

        log::debug!("Fetched {files_in_page} files from page {page}");

        all_files.extend(page_files);

        if all_files.len() >= MAX_FILES {
            log::warn!(
                "PR has {} files, reached GitHub's {} file limit",
                all_files.len(),
                MAX_FILES
            );
            break;
        }

        if let Some(links) = link_header
            && links.next.is_some()
        {
            page += 1;
            continue;
        }

        break;
    }

    log::info!(
        "Fetched total of {} files across {} page(s)",
        all_files.len(),
        page
    );

    Ok(all_files)
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

    #[test]
    fn test_parse_link_header_with_next() {
        let header = r#"<https://api.github.com/repos/o/r/pulls/1/files?page=2>; rel="next", <https://api.github.com/repos/o/r/pulls/1/files?page=3>; rel="last""#;
        let parsed = parse_link_header(header);
        assert!(parsed.next.is_some());
        assert_eq!(
            parsed.next.unwrap(),
            "https://api.github.com/repos/o/r/pulls/1/files?page=2"
        );
    }

    #[test]
    fn test_parse_link_header_without_next() {
        let header = r#"<https://api.github.com/repos/o/r/pulls/1/files?page=1>; rel="first""#;
        let parsed = parse_link_header(header);
        assert!(parsed.next.is_none());
    }

    #[test]
    fn test_parse_link_header_empty() {
        let parsed = parse_link_header("");
        assert!(parsed.next.is_none());
    }

    #[tokio::test]
    async fn test_fetch_all_pr_files_single_page() {
        let mock_server = MockServer::start().await;

        let files = serde_json::json!([
            {
                "filename": "file1.txt",
                "status": "modified",
                "additions": 5,
                "deletions": 2,
                "patch": "@@ -1,3 +1,5 @@\n line1\n+line2\n line3"
            },
            {
                "filename": "file2.txt",
                "status": "added",
                "additions": 10,
                "deletions": 0,
                "patch": "@@ -0,0 +1,10 @@\n+new content"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&files))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = fetch_all_pr_files(&client, &mock_server.uri(), "owner", "repo", 123, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["filename"].as_str().unwrap(), "file1.txt");
        assert_eq!(result[1]["filename"].as_str().unwrap(), "file2.txt");
    }

    #[tokio::test]
    async fn test_fetch_all_pr_files_multiple_pages() {
        let mock_server = MockServer::start().await;

        let page1 = serde_json::json!([
            {
                "filename": "file1.txt",
                "status": "modified",
                "additions": 5,
                "deletions": 2,
                "patch": "@@ -1,3 +1,5 @@\n line1"
            }
        ]);

        let page2 = serde_json::json!([
            {
                "filename": "file2.txt",
                "status": "added",
                "additions": 10,
                "deletions": 0,
                "patch": "@@ -0,0 +1,10 @@\n+new"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/files"))
            .and(wiremock::matchers::query_param("page", "1"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(&page1)
                    .append_header(
                    "Link",
                    format!(
                        r#"<{}/repos/owner/repo/pulls/123/files?per_page=100&page=2>; rel="next""#,
                        mock_server.uri()
                    )
                    .as_str(),
                ),
            )
            .mount(&mock_server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/files"))
            .and(wiremock::matchers::query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&page2))
            .mount(&mock_server)
            .await;

        let client = reqwest::Client::new();
        let result = fetch_all_pr_files(&client, &mock_server.uri(), "owner", "repo", 123, None)
            .await
            .unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0]["filename"].as_str().unwrap(), "file1.txt");
        assert_eq!(result[1]["filename"].as_str().unwrap(), "file2.txt");
    }

    #[tokio::test]
    async fn test_get_diff_with_patch_field() {
        let mock_server = MockServer::start().await;

        let files = serde_json::json!([
            {
                "filename": "src/main.rs",
                "status": "modified",
                "additions": 2,
                "deletions": 1,
                "patch": "@@ -1,3 +1,4 @@\n fn main() {\n-    println!(\"old\");\n+    println!(\"new\");\n+    println!(\"added\");\n }"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&files))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let result = client.get_diff("owner", "repo", 123).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].filename, "src/main.rs");
    }

    #[tokio::test]
    async fn test_get_diff_skips_files_without_patch() {
        let mock_server = MockServer::start().await;

        let files = serde_json::json!([
            {
                "filename": "image.png",
                "status": "added",
                "additions": 0,
                "deletions": 0
            },
            {
                "filename": "src/lib.rs",
                "status": "modified",
                "additions": 5,
                "deletions": 2,
                "patch": "@@ -1,3 +1,6 @@\n code"
            }
        ]);

        Mock::given(method("GET"))
            .and(path("/repos/owner/repo/pulls/123/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&files))
            .mount(&mock_server)
            .await;

        let client = GitHubProvider::new()
            .with_token("test-token".to_string())
            .with_base_url(mock_server.uri());

        let result = client.get_diff("owner", "repo", 123).await.unwrap();

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].filename, "src/lib.rs");
    }
}
