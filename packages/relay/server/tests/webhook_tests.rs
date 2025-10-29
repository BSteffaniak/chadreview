use chadreview_relay_models::*;

#[test]
fn test_parse_github_webhook_issue_comment_json() {
    let json = r#"{
        "action": "created",
        "comment": {
            "id": 12345,
            "body": "Test comment",
            "user": {
                "id": 1,
                "login": "testuser",
                "avatar_url": "https://github.com/testuser.png",
                "html_url": "https://github.com/testuser"
            },
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-15T10:30:00Z"
        },
        "issue": {
            "number": 123,
            "title": "Test Issue",
            "state": "open",
            "pull_request": null
        },
        "repository": {
            "name": "repo",
            "full_name": "owner/repo",
            "owner": {
                "id": 2,
                "login": "owner",
                "avatar_url": "https://github.com/owner.png",
                "html_url": "https://github.com/owner"
            }
        }
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).unwrap();

    assert_eq!(parsed["action"], "created");
    assert_eq!(parsed["comment"]["id"], 12345);
    assert_eq!(parsed["comment"]["body"], "Test comment");
    assert_eq!(parsed["issue"]["number"], 123);
}

#[test]
fn test_parse_github_webhook_pr_review_comment_json() {
    let json = r#"{
        "action": "created",
        "comment": {
            "id": 67890,
            "body": "Review comment",
            "path": "src/main.rs",
            "commit_id": "abc123",
            "original_commit_id": "def456",
            "line": 42,
            "original_line": 40,
            "side": "RIGHT",
            "user": {
                "id": 3,
                "login": "reviewer",
                "avatar_url": "https://github.com/reviewer.png",
                "html_url": "https://github.com/reviewer"
            },
            "created_at": "2024-01-16T14:20:00Z",
            "updated_at": "2024-01-16T14:25:00Z",
            "in_reply_to_id": null
        },
        "pull_request": {
            "number": 42,
            "title": "Test PR",
            "state": "open",
            "head": {
                "ref": "feature-branch",
                "sha": "abc123"
            },
            "base": {
                "ref": "main",
                "sha": "def456"
            }
        },
        "repository": {
            "name": "repo",
            "full_name": "owner/repo",
            "owner": {
                "id": 2,
                "login": "owner",
                "avatar_url": "https://github.com/owner.png",
                "html_url": "https://github.com/owner"
            }
        }
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).unwrap();

    assert_eq!(parsed["action"], "created");
    assert_eq!(parsed["comment"]["id"], 67890);
    assert_eq!(parsed["comment"]["path"], "src/main.rs");
    assert_eq!(parsed["pull_request"]["number"], 42);
}

#[test]
fn test_parse_github_webhook_pull_request_json() {
    let json = r#"{
        "action": "opened",
        "pull_request": {
            "number": 100,
            "title": "New Feature",
            "state": "open",
            "head": {
                "ref": "feature",
                "sha": "abc123"
            },
            "base": {
                "ref": "main",
                "sha": "def456"
            }
        },
        "repository": {
            "name": "repo",
            "full_name": "owner/repo",
            "owner": {
                "id": 1,
                "login": "owner",
                "avatar_url": "https://github.com/owner.png",
                "html_url": "https://github.com/owner"
            }
        }
    }"#;

    let parsed: serde_json::Value = serde_json::from_str(json).unwrap();

    assert_eq!(parsed["action"], "opened");
    assert_eq!(parsed["pull_request"]["number"], 100);
    assert_eq!(parsed["pull_request"]["title"], "New Feature");
}

#[test]
fn test_datetime_parsing_from_github_format() {
    let comment_json = r#"{
        "id": 1,
        "body": "test",
        "user": {
            "id": 1,
            "login": "user",
            "avatar_url": "url",
            "html_url": "url"
        },
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
    }"#;

    let comment: GitHubComment = serde_json::from_str(comment_json).unwrap();

    assert_eq!(comment.id, 1);
    assert_eq!(comment.body, "test");
}

#[test]
fn test_relay_message_serialization() {
    use chrono::{TimeZone, Utc};

    let relay_msg = RelayMessage {
        pr_key: PrKey {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            number: 1,
        },
        event: WebhookEvent::IssueComment {
            action: CommentAction::Created,
            comment: GitHubComment {
                id: 1,
                body: "test".to_string(),
                user: GitHubUser {
                    id: 1,
                    login: "user".to_string(),
                    avatar_url: "url".to_string(),
                    html_url: "url".to_string(),
                },
                created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                updated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            },
            issue: GitHubIssue {
                number: 1,
                title: "Test".to_string(),
                state: "open".to_string(),
                pull_request: None,
            },
            repository: GitHubRepository {
                name: "repo".to_string(),
                owner: GitHubUser {
                    id: 1,
                    login: "owner".to_string(),
                    avatar_url: "url".to_string(),
                    html_url: "url".to_string(),
                },
                full_name: "owner/repo".to_string(),
            },
        },
    };

    let json = serde_json::to_string(&relay_msg).unwrap();
    assert!(json.contains("\"owner\":\"owner\""));
    assert!(json.contains("\"number\":1"));
}

#[test]
fn test_server_message_webhook_with_boxed_relay_message() {
    let server_msg = ServerMessage::Webhook(Box::new(RelayMessage {
        pr_key: PrKey {
            owner: "o".to_string(),
            repo: "r".to_string(),
            number: 1,
        },
        event: WebhookEvent::PullRequest {
            action: PrAction::Opened,
            pull_request: GitHubPullRequest {
                number: 1,
                title: "Test".to_string(),
                state: "open".to_string(),
                head: GitHubRef {
                    ref_name: "f".to_string(),
                    sha: "a".to_string(),
                },
                base: GitHubRef {
                    ref_name: "m".to_string(),
                    sha: "b".to_string(),
                },
            },
            repository: GitHubRepository {
                name: "r".to_string(),
                owner: GitHubUser {
                    id: 1,
                    login: "o".to_string(),
                    avatar_url: "u".to_string(),
                    html_url: "u".to_string(),
                },
                full_name: "o/r".to_string(),
            },
        },
    }));

    let json = serde_json::to_string(&server_msg).unwrap();
    let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(server_msg, deserialized);
}
