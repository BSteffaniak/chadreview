use chadreview_relay_models::*;
use std::collections::HashMap;

#[test]
fn test_pr_key_hash() {
    let mut map = HashMap::new();

    let pr_key = PrKey {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        number: 123,
    };

    map.insert(pr_key.clone(), "test_value");

    assert_eq!(map.get(&pr_key), Some(&"test_value"));
}

#[test]
fn test_pr_key_different_numbers() {
    let pr1 = PrKey {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        number: 1,
    };

    let pr2 = PrKey {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        number: 2,
    };

    assert_ne!(pr1, pr2);
}

#[test]
fn test_comment_action_variants() {
    let actions = vec![
        CommentAction::Created,
        CommentAction::Edited,
        CommentAction::Deleted,
    ];

    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: CommentAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}

#[test]
fn test_pr_action_variants() {
    let actions = vec![
        PrAction::Opened,
        PrAction::Edited,
        PrAction::Closed,
        PrAction::Reopened,
        PrAction::Synchronize,
    ];

    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: PrAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}

#[test]
fn test_github_user_equality() {
    let user1 = GitHubUser {
        id: 1,
        login: "user".to_string(),
        avatar_url: "https://avatar.url".to_string(),
        html_url: "https://github.com/user".to_string(),
    };

    let user2 = GitHubUser {
        id: 1,
        login: "user".to_string(),
        avatar_url: "https://avatar.url".to_string(),
        html_url: "https://github.com/user".to_string(),
    };

    let user3 = GitHubUser {
        id: 2,
        login: "other".to_string(),
        avatar_url: "https://avatar.url".to_string(),
        html_url: "https://github.com/other".to_string(),
    };

    assert_eq!(user1, user2);
    assert_ne!(user1, user3);
}

#[test]
fn test_github_ref_equality() {
    let ref1 = GitHubRef {
        ref_name: "main".to_string(),
        sha: "abc123".to_string(),
    };

    let ref2 = GitHubRef {
        ref_name: "main".to_string(),
        sha: "abc123".to_string(),
    };

    assert_eq!(ref1, ref2);
}

#[test]
fn test_github_repository_with_owner() {
    let repo = GitHubRepository {
        name: "test-repo".to_string(),
        owner: GitHubUser {
            id: 1,
            login: "owner".to_string(),
            avatar_url: "https://avatar.url".to_string(),
            html_url: "https://github.com/owner".to_string(),
        },
        full_name: "owner/test-repo".to_string(),
    };

    assert_eq!(repo.owner.login, "owner");
    assert_eq!(repo.name, "test-repo");
}

#[test]
fn test_webhook_event_pull_request_review_comment_with_boxed_comment() {
    use chrono::{TimeZone, Utc};

    let event = WebhookEvent::PullRequestReviewComment {
        action: CommentAction::Created,
        comment: Box::new(GitHubReviewComment {
            id: 1,
            body: "LGTM".to_string(),
            path: "src/lib.rs".to_string(),
            commit_id: "abc".to_string(),
            original_commit_id: "def".to_string(),
            line: Some(10),
            original_line: Some(10),
            side: Some("RIGHT".to_string()),
            user: GitHubUser {
                id: 1,
                login: "reviewer".to_string(),
                avatar_url: "url".to_string(),
                html_url: "url".to_string(),
            },
            created_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            in_reply_to_id: None,
        }),
        pull_request: GitHubPullRequest {
            number: 42,
            title: "Test".to_string(),
            state: "open".to_string(),
            head: GitHubRef {
                ref_name: "feature".to_string(),
                sha: "abc".to_string(),
            },
            base: GitHubRef {
                ref_name: "main".to_string(),
                sha: "def".to_string(),
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
    };

    let json = serde_json::to_string(&event).unwrap();
    let deserialized: WebhookEvent = serde_json::from_str(&json).unwrap();

    assert_eq!(event, deserialized);
}
