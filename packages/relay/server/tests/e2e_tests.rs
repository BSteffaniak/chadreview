mod helpers;

use chadreview_relay_client::RelayClient;
use chadreview_relay_models::{CommentAction, PrAction, PrKey, WebhookEvent};
use chadreview_relay_testing::{WebhookBuilder, WebhookSender};
use helpers::TestRelayServer;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[test_log::test(tokio::test)]
async fn test_full_webhook_relay_flow() {
    env_logger::try_init().ok();

    let server = TestRelayServer::start().await.unwrap();
    println!("Test server started on port {}", server.port());

    let instance_id = "test-instance-123";
    let client = RelayClient::connect_async(server.ws_url(), instance_id.to_string())
        .await
        .unwrap();

    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let pr_key = PrKey {
        owner: "octocat".to_string(),
        repo: "hello-world".to_string(),
        number: 123,
    };

    client
        .subscribe(
            pr_key.clone(),
            Arc::new(move |event| {
                if let Some(sender) = tx.lock().unwrap().take() {
                    let _ = sender.send(event);
                }
            }),
        )
        .await
        .unwrap();

    let builder = WebhookBuilder::new("octocat", "hello-world", 123);
    let payload = builder.build_issue_comment(CommentAction::Created, "LGTM!");

    let sender = WebhookSender::new(server.http_url());
    sender
        .send_webhook(instance_id, "issue_comment", payload, None)
        .await
        .unwrap();

    let received_event = tokio::time::timeout(Duration::from_secs(5), rx)
        .await
        .expect("Timeout waiting for webhook")
        .expect("Failed to receive webhook");

    match received_event {
        WebhookEvent::IssueComment {
            action,
            comment,
            issue,
            repository,
        } => {
            assert_eq!(action, CommentAction::Created);
            assert_eq!(comment.body, "LGTM!");
            assert_eq!(issue.number, 123);
            assert_eq!(repository.name, "hello-world");
        }
        _ => panic!("Expected IssueComment event"),
    }
}

#[test_log::test(tokio::test)]
async fn test_multiple_clients_receive_same_webhook() {
    env_logger::try_init().ok();

    let server = TestRelayServer::start().await.unwrap();

    let client1 = RelayClient::connect_async(server.ws_url(), "instance-1".to_string())
        .await
        .unwrap();
    let client2 = RelayClient::connect_async(server.ws_url(), "instance-2".to_string())
        .await
        .unwrap();

    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let (tx2, rx2) = tokio::sync::oneshot::channel();
    let tx1 = Arc::new(Mutex::new(Some(tx1)));
    let tx2 = Arc::new(Mutex::new(Some(tx2)));

    let pr_key = PrKey {
        owner: "octocat".to_string(),
        repo: "hello-world".to_string(),
        number: 456,
    };

    client1
        .subscribe(
            pr_key.clone(),
            Arc::new(move |event| {
                if let Some(sender) = tx1.lock().unwrap().take() {
                    let _ = sender.send(event);
                }
            }),
        )
        .await
        .unwrap();

    client2
        .subscribe(
            pr_key.clone(),
            Arc::new(move |event| {
                if let Some(sender) = tx2.lock().unwrap().take() {
                    let _ = sender.send(event);
                }
            }),
        )
        .await
        .unwrap();

    let builder = WebhookBuilder::new("octocat", "hello-world", 456);
    let payload = builder.build_pull_request(PrAction::Opened);

    let sender = WebhookSender::new(server.http_url());
    sender
        .send_webhook("instance-1", "pull_request", payload, None)
        .await
        .unwrap();

    let (event1, event2) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(5), rx1),
        tokio::time::timeout(Duration::from_secs(5), rx2),
    );

    assert!(event1.is_ok());
    assert!(event2.is_ok());

    match event1.unwrap().unwrap() {
        WebhookEvent::PullRequest {
            action,
            pull_request,
            ..
        } => {
            assert_eq!(action, PrAction::Opened);
            assert_eq!(pull_request.number, 456);
        }
        _ => panic!("Expected PullRequest event for client1"),
    }

    match event2.unwrap().unwrap() {
        WebhookEvent::PullRequest {
            action,
            pull_request,
            ..
        } => {
            assert_eq!(action, PrAction::Opened);
            assert_eq!(pull_request.number, 456);
        }
        _ => panic!("Expected PullRequest event for client2"),
    }
}

#[test_log::test(tokio::test)]
async fn test_webhook_signature_verification() {
    env_logger::try_init().ok();

    let server = TestRelayServer::start_with_secret(Some("test-secret".to_string()))
        .await
        .unwrap();
    let instance_id = "test-instance";

    let builder = WebhookBuilder::new("octocat", "hello-world", 789);
    let payload = builder.build_issue_comment(CommentAction::Created, "Test");

    let sender = WebhookSender::new(server.http_url());

    let result_with_valid_secret = sender
        .send_webhook(
            instance_id,
            "issue_comment",
            payload.clone(),
            Some("test-secret"),
        )
        .await;
    assert!(result_with_valid_secret.is_ok());

    let result_with_wrong_secret = sender
        .send_webhook(
            instance_id,
            "issue_comment",
            payload.clone(),
            Some("wrong-secret"),
        )
        .await;
    assert!(result_with_wrong_secret.is_err());
}

#[test_log::test(tokio::test)]
async fn test_unsubscribe_stops_receiving_webhooks() {
    env_logger::try_init().ok();

    let server = TestRelayServer::start().await.unwrap();
    let instance_id = "test-instance";
    let client = RelayClient::connect_async(server.ws_url(), instance_id.to_string())
        .await
        .unwrap();

    let (tx, mut rx) = tokio::sync::mpsc::channel(10);
    let tx = Arc::new(tx);

    let pr_key = PrKey {
        owner: "octocat".to_string(),
        repo: "hello-world".to_string(),
        number: 999,
    };

    let tx_clone = tx.clone();
    client
        .subscribe(
            pr_key.clone(),
            Arc::new(move |event| {
                let _ = tx_clone.try_send(event);
            }),
        )
        .await
        .unwrap();

    let builder = WebhookBuilder::new("octocat", "hello-world", 999);
    let payload = builder.build_issue_comment(CommentAction::Created, "First");

    let sender = WebhookSender::new(server.http_url());
    sender
        .send_webhook(instance_id, "issue_comment", payload, None)
        .await
        .unwrap();

    let event = rx.recv().await.expect("Should receive first webhook");
    match event {
        WebhookEvent::IssueComment { comment, .. } => {
            assert_eq!(comment.body, "First");
        }
        _ => panic!("Expected IssueComment event"),
    }

    // Ensure channel is empty before unsubscribing
    assert!(
        rx.try_recv().is_err(),
        "Channel should be empty after first event"
    );

    client.unsubscribe(&pr_key).await.unwrap();

    // Drop the original sender so we can detect if callback is invoked
    drop(tx);

    let payload2 = builder.build_issue_comment(CommentAction::Created, "Second");
    sender
        .send_webhook(instance_id, "issue_comment", payload2, None)
        .await
        .unwrap();

    let no_event = tokio::time::timeout(Duration::from_millis(100), rx.recv()).await;
    match no_event {
        Ok(None) => {
            // Channel closed, which is expected since we dropped tx
        }
        Ok(Some(event)) => {
            panic!("Received unexpected event after unsubscribe: {event:?}");
        }
        Err(_) => {
            // Timeout is also acceptable
        }
    }
}

#[test_log::test(tokio::test)]
async fn test_different_event_types() {
    env_logger::try_init().ok();

    let server = TestRelayServer::start().await.unwrap();
    let instance_id = "test-instance";
    let client = RelayClient::connect_async(server.ws_url(), instance_id.to_string())
        .await
        .unwrap();

    let pr_key = PrKey {
        owner: "octocat".to_string(),
        repo: "hello-world".to_string(),
        number: 111,
    };

    let (tx1, rx1) = tokio::sync::oneshot::channel();
    let tx1 = Arc::new(Mutex::new(Some(tx1)));

    client
        .subscribe(
            pr_key.clone(),
            Arc::new(move |event| {
                if let Some(sender) = tx1.lock().unwrap().take() {
                    let _ = sender.send(event);
                }
            }),
        )
        .await
        .unwrap();

    let builder = WebhookBuilder::new("octocat", "hello-world", 111);
    let payload = builder.build_review_comment(CommentAction::Created, "Review", "src/main.rs", 42);

    let sender = WebhookSender::new(server.http_url());
    sender
        .send_webhook(instance_id, "pull_request_review_comment", payload, None)
        .await
        .unwrap();

    let received_event = tokio::time::timeout(Duration::from_secs(5), rx1)
        .await
        .expect("Timeout waiting for webhook")
        .expect("Failed to receive webhook");

    match received_event {
        WebhookEvent::PullRequestReviewComment {
            action,
            comment,
            pull_request,
            ..
        } => {
            assert_eq!(action, CommentAction::Created);
            assert_eq!(comment.body, "Review");
            assert_eq!(comment.path, "src/main.rs");
            assert_eq!(comment.line, Some(42));
            assert_eq!(pull_request.number, 111);
        }
        _ => panic!("Expected PullRequestReviewComment event"),
    }
}
