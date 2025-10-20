use crate::diff_parser::parse_unified_diff;
use anyhow::Result;
use chadreview_git_provider::GitProvider;
use chadreview_pr_models::{
    Comment, CreateComment, DiffFile, FileStatus, Label, PrState, PullRequest, User,
};
use chadreview_syntax::SyntaxHighlighter;

pub struct GitHubProvider {
    http_client: reqwest::Client,
    auth_token: String,
    base_url: String,
}

impl GitHubProvider {
    #[must_use]
    pub fn new(auth_token: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            auth_token,
            base_url: "https://api.github.com".to_string(),
        }
    }

    #[must_use]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

#[async_trait::async_trait]
impl GitProvider for GitHubProvider {
    async fn get_pr(&self, owner: &str, repo: &str, number: u64) -> Result<PullRequest> {
        let url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&self.auth_token)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if !response.status().is_success() {
            anyhow::bail!("GitHub API error: {}", response.status());
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
        let files_response = self
            .http_client
            .get(&files_url)
            .bearer_auth(&self.auth_token)
            .header("Accept", "application/vnd.github.v3+json")
            .send()
            .await?;

        if !files_response.status().is_success() {
            anyhow::bail!("GitHub API error: {}", files_response.status());
        }

        let files_data: Vec<serde_json::Value> = files_response.json().await?;

        let diff_url = format!(
            "{}/repos/{}/{}/pulls/{}",
            self.base_url, owner, repo, number
        );
        let diff_response = self
            .http_client
            .get(&diff_url)
            .bearer_auth(&self.auth_token)
            .header("Accept", "application/vnd.github.v3.diff")
            .send()
            .await?;

        if !diff_response.status().is_success() {
            anyhow::bail!("GitHub API error: {}", diff_response.status());
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

    async fn get_comments(&self, _owner: &str, _repo: &str, _number: u64) -> Result<Vec<Comment>> {
        todo!("Implement in Phase 5")
    }

    async fn create_comment(
        &self,
        _owner: &str,
        _repo: &str,
        _number: u64,
        _comment: CreateComment,
    ) -> Result<Comment> {
        todo!("Implement in Phase 6")
    }

    async fn update_comment(&self, _comment_id: u64, _body: String) -> Result<Comment> {
        todo!("Implement in Phase 6")
    }

    async fn delete_comment(&self, _comment_id: u64) -> Result<()> {
        todo!("Implement in Phase 6")
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

        let client = GitHubProvider::new("test-token".to_string()).with_base_url(mock_server.uri());

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

        let client = GitHubProvider::new("test-token".to_string()).with_base_url(mock_server.uri());

        let pr = client.get_pr("owner", "repo", 456).await.unwrap();

        assert_eq!(pr.state, PrState::Merged);
    }
}
