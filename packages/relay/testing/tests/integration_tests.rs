use chadreview_relay_models::{CommentAction, PrAction};
use chadreview_relay_testing::WebhookBuilder;
use chrono::Utc;

#[test]
fn test_issue_comment_webhook_structure() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 456);
    let payload = builder.build_issue_comment(CommentAction::Created, "Test comment body");

    assert_eq!(payload["action"], "created");

    let comment = &payload["comment"];
    assert_eq!(comment["id"], 1_234_567_890_u64);
    assert_eq!(comment["body"], "Test comment body");
    assert_eq!(comment["user"]["login"], "test-user");
    assert_eq!(comment["user"]["id"], 12345);
    assert!(comment["created_at"].is_string());
    assert!(comment["updated_at"].is_string());

    let issue = &payload["issue"];
    assert_eq!(issue["number"], 456);
    assert_eq!(issue["title"], "Test Pull Request");
    assert_eq!(issue["state"], "open");
    assert!(issue["pull_request"].is_object());

    let repo = &payload["repository"];
    assert_eq!(repo["name"], "test-repo");
    assert_eq!(repo["full_name"], "test-owner/test-repo");
    assert_eq!(repo["owner"]["login"], "test-owner");
}

#[test]
fn test_review_comment_webhook_structure() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 789);
    let payload =
        builder.build_review_comment(CommentAction::Created, "Review body", "src/lib.rs", 100);

    assert_eq!(payload["action"], "created");

    let comment = &payload["comment"];
    assert_eq!(comment["id"], 1_234_567_891_u64);
    assert_eq!(comment["body"], "Review body");
    assert_eq!(comment["path"], "src/lib.rs");
    assert_eq!(comment["line"], 100);
    assert_eq!(comment["original_line"], 100);
    assert_eq!(comment["side"], "RIGHT");
    assert_eq!(comment["commit_id"], "abc123def456");
    assert_eq!(comment["original_commit_id"], "abc123def456");
    assert_eq!(comment["user"]["login"], "test-user");
    assert_eq!(comment["user"]["id"], 12345);
    assert!(comment["in_reply_to_id"].is_null());

    let pr = &payload["pull_request"];
    assert_eq!(pr["number"], 789);
    assert_eq!(pr["title"], "Test Pull Request");
    assert_eq!(pr["state"], "open");

    let repo = &payload["repository"];
    assert_eq!(repo["name"], "test-repo");
    assert_eq!(repo["full_name"], "test-owner/test-repo");
}

#[test]
fn test_pull_request_webhook_structure() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 321);
    let payload = builder.build_pull_request(PrAction::Opened);

    assert_eq!(payload["action"], "opened");

    let pr = &payload["pull_request"];
    assert_eq!(pr["number"], 321);
    assert_eq!(pr["title"], "Test Pull Request");
    assert_eq!(pr["state"], "open");
    assert_eq!(pr["head"]["ref"], "feature-branch");
    assert_eq!(pr["head"]["sha"], "abc123def456");
    assert_eq!(pr["base"]["ref"], "main");
    assert_eq!(pr["base"]["sha"], "def456abc123");

    let repo = &payload["repository"];
    assert_eq!(repo["name"], "test-repo");
    assert_eq!(repo["full_name"], "test-owner/test-repo");
    assert_eq!(repo["owner"]["login"], "test-owner");
    assert_eq!(repo["owner"]["id"], 1);
}

#[test]
fn test_custom_user_in_webhooks() {
    let builder =
        WebhookBuilder::new("test-owner", "test-repo", 123).with_user("custom-user", 88888);

    let issue_payload = builder.build_issue_comment(CommentAction::Created, "Test");
    assert_eq!(issue_payload["comment"]["user"]["login"], "custom-user");
    assert_eq!(issue_payload["comment"]["user"]["id"], 88888);
    assert_eq!(issue_payload["repository"]["owner"]["login"], "test-owner");
    assert_eq!(issue_payload["repository"]["owner"]["id"], 1);

    let review_payload =
        builder.build_review_comment(CommentAction::Created, "Test", "src/main.rs", 10);
    assert_eq!(review_payload["comment"]["user"]["login"], "custom-user");
    assert_eq!(review_payload["comment"]["user"]["id"], 88888);

    let pr_payload = builder.build_pull_request(PrAction::Opened);
    assert_eq!(pr_payload["repository"]["owner"]["login"], "test-owner");
    assert_eq!(pr_payload["repository"]["owner"]["id"], 1);
}

#[test]
fn test_custom_timestamp() {
    let timestamp = Utc::now();
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123).with_timestamp(timestamp);

    let payload = builder.build_issue_comment(CommentAction::Created, "Test");

    let created_at = payload["comment"]["created_at"].as_str().unwrap();
    let updated_at = payload["comment"]["updated_at"].as_str().unwrap();

    assert!(!created_at.is_empty());
    assert!(!updated_at.is_empty());
    assert!(created_at.contains(&timestamp.format("%Y-%m-%d").to_string()));
}

#[test]
fn test_different_comment_actions() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123);

    let created = builder.build_issue_comment(CommentAction::Created, "Test");
    assert_eq!(created["action"], "created");

    let edited = builder.build_issue_comment(CommentAction::Edited, "Test");
    assert_eq!(edited["action"], "edited");

    let deleted = builder.build_issue_comment(CommentAction::Deleted, "Test");
    assert_eq!(deleted["action"], "deleted");
}

#[test]
fn test_different_pr_actions() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123);

    let opened = builder.build_pull_request(PrAction::Opened);
    assert_eq!(opened["action"], "opened");

    let edited = builder.build_pull_request(PrAction::Edited);
    assert_eq!(edited["action"], "edited");

    let closed = builder.build_pull_request(PrAction::Closed);
    assert_eq!(closed["action"], "closed");

    let reopened = builder.build_pull_request(PrAction::Reopened);
    assert_eq!(reopened["action"], "reopened");

    let synchronize = builder.build_pull_request(PrAction::Synchronize);
    assert_eq!(synchronize["action"], "synchronize");
}

#[test]
fn test_repository_full_name() {
    let builder = WebhookBuilder::new("octocat", "hello-world", 123);
    let payload = builder.build_issue_comment(CommentAction::Created, "Test");

    assert_eq!(payload["repository"]["name"], "hello-world");
    assert_eq!(payload["repository"]["full_name"], "octocat/hello-world");
}

#[test]
fn test_user_urls() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123).with_user("testuser", 12345);
    let payload = builder.build_issue_comment(CommentAction::Created, "Test");

    let user = &payload["comment"]["user"];
    assert_eq!(user["login"], "testuser");
    assert_eq!(user["id"], 12345);
    assert_eq!(
        user["avatar_url"],
        "https://avatars.githubusercontent.com/u/12345?v=4"
    );
    assert_eq!(user["html_url"], "https://github.com/testuser");
}

#[test]
fn test_review_comment_fields() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123);
    let payload =
        builder.build_review_comment(CommentAction::Created, "Fix this bug", "src/main.rs", 42);

    let comment = &payload["comment"];
    assert_eq!(comment["id"], 1_234_567_891_u64);
    assert_eq!(comment["body"], "Fix this bug");
    assert_eq!(comment["path"], "src/main.rs");
    assert_eq!(comment["line"], 42);
    assert_eq!(comment["original_line"], 42);
    assert_eq!(comment["side"], "RIGHT");
    assert_eq!(comment["commit_id"], "abc123def456");
    assert_eq!(comment["original_commit_id"], "abc123def456");
    assert_eq!(comment["user"]["login"], "test-user");
    assert_eq!(comment["user"]["id"], 12345);
}

#[test]
fn test_pull_request_refs() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123);
    let payload = builder.build_pull_request(PrAction::Opened);

    let pr = &payload["pull_request"];
    assert_eq!(pr["head"]["ref"], "feature-branch");
    assert_eq!(pr["head"]["sha"], "abc123def456");
    assert_eq!(pr["base"]["ref"], "main");
    assert_eq!(pr["base"]["sha"], "def456abc123");
    assert_eq!(pr["number"], 123);
    assert_eq!(pr["title"], "Test Pull Request");
    assert_eq!(pr["state"], "open");
}

#[test]
fn test_issue_has_pull_request_marker() {
    let builder = WebhookBuilder::new("test-owner", "test-repo", 123);
    let payload = builder.build_issue_comment(CommentAction::Created, "Test");

    let issue = &payload["issue"];
    assert_eq!(issue["number"], 123);
    assert_eq!(issue["title"], "Test Pull Request");
    assert_eq!(issue["state"], "open");
    assert!(issue["pull_request"].is_object());
}
