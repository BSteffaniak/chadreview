use chadreview_relay_models::*;
use chrono::{TimeZone, Utc};

#[test]
fn test_github_comment_roundtrip() {
    let original = GitHubComment {
        id: 12345,
        body: "Test comment".to_string(),
        user: GitHubUser {
            id: 1,
            login: "testuser".to_string(),
            avatar_url: "https://avatar.url".to_string(),
            html_url: "https://github.com/testuser".to_string(),
        },
        created_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap(),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: GitHubComment = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_github_comment_from_real_timestamp() {
    let json = r#"{
        "id": 12345,
        "body": "Test comment",
        "user": {
            "id": 1,
            "login": "testuser",
            "avatar_url": "https://avatar.url",
            "html_url": "https://github.com/testuser"
        },
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-01-15T10:30:00Z"
    }"#;

    let comment: GitHubComment = serde_json::from_str(json).unwrap();

    assert_eq!(comment.id, 12345);
    assert_eq!(comment.body, "Test comment");
    assert_eq!(comment.user.login, "testuser");
    assert_eq!(
        comment.created_at,
        Utc.with_ymd_and_hms(2024, 1, 15, 10, 30, 0).unwrap()
    );
}

#[test]
fn test_github_review_comment_roundtrip() {
    let original = GitHubReviewComment {
        id: 67890,
        body: "Review comment".to_string(),
        path: "src/main.rs".to_string(),
        commit_id: "abc123".to_string(),
        original_commit_id: "def456".to_string(),
        line: Some(42),
        original_line: Some(40),
        side: Some("RIGHT".to_string()),
        user: GitHubUser {
            id: 2,
            login: "reviewer".to_string(),
            avatar_url: "https://avatar2.url".to_string(),
            html_url: "https://github.com/reviewer".to_string(),
        },
        created_at: Utc.with_ymd_and_hms(2024, 1, 16, 14, 20, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2024, 1, 16, 14, 25, 0).unwrap(),
        in_reply_to_id: None,
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: GitHubReviewComment = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}

#[test]
fn test_pr_key_equality() {
    let pr1 = PrKey {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        number: 123,
    };

    let pr2 = PrKey {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        number: 123,
    };

    let pr3 = PrKey {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        number: 456,
    };

    assert_eq!(pr1, pr2);
    assert_ne!(pr1, pr3);
}

#[test]
fn test_webhook_event_issue_comment_roundtrip() {
    let event = WebhookEvent::IssueComment {
        action: CommentAction::Created,
        comment: GitHubComment {
            id: 1,
            body: "Test".to_string(),
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
            number: 123,
            title: "Test Issue".to_string(),
            state: "open".to_string(),
            pull_request: None,
        },
        repository: GitHubRepository {
            name: "repo".to_string(),
            owner: GitHubUser {
                id: 2,
                login: "owner".to_string(),
                avatar_url: "url".to_string(),
                html_url: "url".to_string(),
            },
            full_name: "owner/repo".to_string(),
        },
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: WebhookEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(event, deserialized);
}

#[test]
fn test_comment_action_copy() {
    let action1 = CommentAction::Created;
    let action2 = action1;
    assert_eq!(action1, action2);
}

#[test]
fn test_pr_action_copy() {
    let action1 = PrAction::Opened;
    let action2 = action1;
    assert_eq!(action1, action2);
}

#[test]
fn test_relay_message_roundtrip() {
    let relay_msg = RelayMessage {
        pr_key: PrKey {
            owner: "owner".to_string(),
            repo: "repo".to_string(),
            number: 42,
        },
        event: WebhookEvent::PullRequest {
            action: PrAction::Opened,
            pull_request: GitHubPullRequest {
                number: 42,
                title: "Test PR".to_string(),
                state: "open".to_string(),
                head: GitHubRef {
                    ref_name: "feature-branch".to_string(),
                    sha: "abc123".to_string(),
                },
                base: GitHubRef {
                    ref_name: "main".to_string(),
                    sha: "def456".to_string(),
                },
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
    let deserialized: RelayMessage = serde_json::from_str(&json).unwrap();

    assert_eq!(relay_msg, deserialized);
}

#[test]
fn test_client_message_roundtrip() {
    let messages = vec![
        ClientMessage::Ping,
        ClientMessage::Subscribe(SubscribeMessage {
            pr_key: PrKey {
                owner: "test".to_string(),
                repo: "repo".to_string(),
                number: 1,
            },
        }),
        ClientMessage::Unsubscribe(UnsubscribeMessage {
            pr_key: PrKey {
                owner: "test".to_string(),
                repo: "repo".to_string(),
                number: 1,
            },
        }),
    ];

    for msg in messages {
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ClientMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}

#[test]
fn test_server_message_roundtrip() {
    let pr_key = PrKey {
        owner: "test".to_string(),
        repo: "repo".to_string(),
        number: 1,
    };

    let messages = vec![
        ServerMessage::Pong,
        ServerMessage::Subscribed {
            pr_key: pr_key.clone(),
        },
        ServerMessage::Unsubscribed { pr_key },
    ];

    for msg in messages {
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: ServerMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}
