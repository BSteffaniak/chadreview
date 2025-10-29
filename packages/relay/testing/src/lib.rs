#![warn(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
#![allow(clippy::multiple_crate_versions, clippy::cargo_common_metadata)]

use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde_json::json;
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct WebhookBuilder {
    owner: String,
    repo: String,
    pr_number: u64,
    user_login: String,
    user_id: u64,
    timestamp: DateTime<Utc>,
}

impl WebhookBuilder {
    #[must_use]
    pub fn new(owner: &str, repo: &str, pr_number: u64) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            pr_number,
            user_login: "test-user".to_string(),
            user_id: 12345,
            timestamp: Utc::now(),
        }
    }

    #[must_use]
    pub fn with_user(mut self, login: &str, id: u64) -> Self {
        self.user_login = login.to_string();
        self.user_id = id;
        self
    }

    #[must_use]
    pub const fn with_timestamp(mut self, timestamp: DateTime<Utc>) -> Self {
        self.timestamp = timestamp;
        self
    }

    #[must_use]
    pub fn build_issue_comment(
        &self,
        action: chadreview_relay_models::CommentAction,
        body: &str,
    ) -> serde_json::Value {
        json!({
            "action": action,
            "comment": {
                "id": 1_234_567_890_u64,
                "body": body,
                "user": self.build_user(),
                "created_at": self.timestamp,
                "updated_at": self.timestamp,
            },
            "issue": {
                "number": self.pr_number,
                "title": "Test Pull Request",
                "state": "open",
                "pull_request": {}
            },
            "repository": self.build_repository(),
        })
    }

    #[must_use]
    pub fn build_review_comment(
        &self,
        action: chadreview_relay_models::CommentAction,
        body: &str,
        path: &str,
        line: u64,
    ) -> serde_json::Value {
        json!({
            "action": action,
            "comment": {
                "id": 1_234_567_891_u64,
                "body": body,
                "path": path,
                "commit_id": "abc123def456",
                "original_commit_id": "abc123def456",
                "line": line,
                "original_line": line,
                "side": "RIGHT",
                "user": self.build_user(),
                "created_at": self.timestamp,
                "updated_at": self.timestamp,
                "in_reply_to_id": null,
            },
            "pull_request": self.build_pull_request_data(),
            "repository": self.build_repository(),
        })
    }

    #[must_use]
    pub fn build_pull_request(
        &self,
        action: chadreview_relay_models::PrAction,
    ) -> serde_json::Value {
        json!({
            "action": action,
            "pull_request": self.build_pull_request_data(),
            "repository": self.build_repository(),
        })
    }

    fn build_user(&self) -> serde_json::Value {
        json!({
            "id": self.user_id,
            "login": self.user_login,
            "avatar_url": format!("https://avatars.githubusercontent.com/u/{}?v=4", self.user_id),
            "html_url": format!("https://github.com/{}", self.user_login),
        })
    }

    fn build_repository(&self) -> serde_json::Value {
        json!({
            "name": self.repo,
            "owner": {
                "id": 1,
                "login": self.owner,
                "avatar_url": format!("https://avatars.githubusercontent.com/u/1?v=4"),
                "html_url": format!("https://github.com/{}", self.owner),
            },
            "full_name": format!("{}/{}", self.owner, self.repo),
        })
    }

    fn build_pull_request_data(&self) -> serde_json::Value {
        json!({
            "number": self.pr_number,
            "title": "Test Pull Request",
            "state": "open",
            "head": {
                "ref": "feature-branch",
                "sha": "abc123def456",
            },
            "base": {
                "ref": "main",
                "sha": "def456abc123",
            },
        })
    }
}

pub struct WebhookSender {
    client: reqwest::Client,
    base_url: String,
}

impl WebhookSender {
    #[must_use]
    pub fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_string(),
        }
    }

    /// Send a webhook to the relay server
    ///
    /// # Errors
    /// Returns an error if the HTTP request fails or the server returns an error
    pub async fn send_webhook(
        &self,
        event_type: &str,
        payload: serde_json::Value,
        secret: Option<&str>,
    ) -> anyhow::Result<reqwest::Response> {
        let url = format!("{}/webhook", self.base_url);
        let body = serde_json::to_vec(&payload)?;

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("X-GitHub-Event", event_type)
            .body(body.clone());

        if let Some(secret) = secret {
            let signature = Self::sign_payload(&body, secret);
            request = request.header("X-Hub-Signature-256", format!("sha256={signature}"));
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            anyhow::bail!("Server returned error: {}", response.status());
        }

        Ok(response)
    }

    fn sign_payload(payload: &[u8], secret: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC can take key of any size");
        mac.update(payload);
        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chadreview_relay_models::{CommentAction, PrAction};

    #[test]
    fn test_build_issue_comment() {
        let builder = WebhookBuilder::new("octocat", "hello-world", 123);
        let payload = builder.build_issue_comment(CommentAction::Created, "LGTM!");

        assert_eq!(payload["action"], "created");

        assert_eq!(payload["comment"]["id"], 1_234_567_890_u64);
        assert_eq!(payload["comment"]["body"], "LGTM!");
        assert_eq!(payload["comment"]["user"]["login"], "test-user");
        assert_eq!(payload["comment"]["user"]["id"], 12345);
        assert!(payload["comment"]["created_at"].is_string());
        assert!(payload["comment"]["updated_at"].is_string());

        assert_eq!(payload["issue"]["number"], 123);
        assert_eq!(payload["issue"]["title"], "Test Pull Request");
        assert_eq!(payload["issue"]["state"], "open");

        assert_eq!(payload["repository"]["name"], "hello-world");
        assert_eq!(payload["repository"]["full_name"], "octocat/hello-world");
        assert_eq!(payload["repository"]["owner"]["login"], "octocat");
    }

    #[test]
    fn test_build_review_comment() {
        let builder = WebhookBuilder::new("octocat", "hello-world", 123);
        let payload =
            builder.build_review_comment(CommentAction::Created, "Fix this", "src/main.rs", 42);

        assert_eq!(payload["action"], "created");

        assert_eq!(payload["comment"]["id"], 1_234_567_891_u64);
        assert_eq!(payload["comment"]["body"], "Fix this");
        assert_eq!(payload["comment"]["path"], "src/main.rs");
        assert_eq!(payload["comment"]["line"], 42);
        assert_eq!(payload["comment"]["original_line"], 42);
        assert_eq!(payload["comment"]["side"], "RIGHT");
        assert_eq!(payload["comment"]["commit_id"], "abc123def456");
        assert_eq!(payload["comment"]["user"]["login"], "test-user");
        assert!(payload["comment"]["in_reply_to_id"].is_null());

        assert_eq!(payload["pull_request"]["number"], 123);
        assert_eq!(payload["pull_request"]["title"], "Test Pull Request");

        assert_eq!(payload["repository"]["name"], "hello-world");
        assert_eq!(payload["repository"]["full_name"], "octocat/hello-world");
    }

    #[test]
    fn test_build_pull_request() {
        let builder = WebhookBuilder::new("octocat", "hello-world", 123);
        let payload = builder.build_pull_request(PrAction::Opened);

        assert_eq!(payload["action"], "opened");

        assert_eq!(payload["pull_request"]["number"], 123);
        assert_eq!(payload["pull_request"]["title"], "Test Pull Request");
        assert_eq!(payload["pull_request"]["state"], "open");
        assert_eq!(payload["pull_request"]["head"]["ref"], "feature-branch");
        assert_eq!(payload["pull_request"]["head"]["sha"], "abc123def456");
        assert_eq!(payload["pull_request"]["base"]["ref"], "main");
        assert_eq!(payload["pull_request"]["base"]["sha"], "def456abc123");

        assert_eq!(payload["repository"]["name"], "hello-world");
        assert_eq!(payload["repository"]["full_name"], "octocat/hello-world");
        assert_eq!(payload["repository"]["owner"]["login"], "octocat");
    }

    #[test]
    fn test_with_custom_user() {
        let builder =
            WebhookBuilder::new("octocat", "hello-world", 123).with_user("custom-user", 99999);
        let payload = builder.build_issue_comment(CommentAction::Created, "Test");

        assert_eq!(payload["comment"]["user"]["login"], "custom-user");
        assert_eq!(payload["comment"]["user"]["id"], 99999);
        assert_eq!(
            payload["comment"]["user"]["avatar_url"],
            "https://avatars.githubusercontent.com/u/99999?v=4"
        );
        assert_eq!(
            payload["comment"]["user"]["html_url"],
            "https://github.com/custom-user"
        );
        assert_eq!(payload["repository"]["owner"]["login"], "octocat");
        assert_eq!(payload["repository"]["owner"]["id"], 1);
    }

    #[test]
    fn test_sign_payload() {
        let payload = b"test payload";
        let signature = WebhookSender::sign_payload(payload, "secret");

        assert_eq!(signature.len(), 64);
        assert!(signature.chars().all(|c| c.is_ascii_hexdigit()));
        assert_eq!(
            signature,
            "f1f1fc517bb886ad22c56e51dae135aad082b2e3337bed35e2e44cd299324bd8"
        );
    }

    #[test]
    fn test_sign_payload_consistency() {
        let payload = b"test payload";
        let sig1 = WebhookSender::sign_payload(payload, "secret");
        let sig2 = WebhookSender::sign_payload(payload, "secret");

        assert_eq!(sig1, sig2);
        assert_eq!(
            sig1,
            "f1f1fc517bb886ad22c56e51dae135aad082b2e3337bed35e2e44cd299324bd8"
        );
    }

    #[test]
    fn test_sign_payload_different_secrets() {
        let payload = b"test payload";
        let sig1 = WebhookSender::sign_payload(payload, "secret1");
        let sig2 = WebhookSender::sign_payload(payload, "secret2");

        assert_ne!(sig1, sig2);
    }
}
