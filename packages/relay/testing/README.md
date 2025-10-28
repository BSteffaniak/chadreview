# ChadReview Relay Testing

Testing utilities and CLI tool for sending mock GitHub webhook events to a relay server.

## Library Usage

```rust
use chadreview_relay_testing::{WebhookBuilder, WebhookSender};
use chadreview_relay_models::CommentAction;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let builder = WebhookBuilder::new("octocat", "hello-world", 123);
    let payload = builder.build_issue_comment(CommentAction::Created, "LGTM!");

    let sender = WebhookSender::new("http://localhost:8080");
    let response = sender.send_webhook(
        "test-instance",
        "issue_comment",
        payload,
        Some("my-webhook-secret")
    ).await?;

    println!("Response: {}", response.status());
    Ok(())
}
```

## CLI Usage

### Send an Issue Comment Event

```bash
cargo run -p chadreview_relay_testing -- \
  --url http://localhost:8080 \
  --instance-id test-instance \
  issue-comment \
  --owner octocat \
  --repo hello-world \
  --pr 123 \
  --body "LGTM!"
```

### Send a Review Comment Event

```bash
cargo run -p chadreview_relay_testing -- \
  --url http://localhost:8080 \
  --instance-id test-instance \
  review-comment \
  --owner octocat \
  --repo hello-world \
  --pr 123 \
  --path src/main.rs \
  --line 42 \
  --body "Consider using Result here"
```

### Send a Pull Request Event

```bash
cargo run -p chadreview_relay_testing -- \
  --url http://localhost:8080 \
  --instance-id test-instance \
  pull-request \
  --owner octocat \
  --repo hello-world \
  --pr 123 \
  --action opened
```

### With Webhook Secret (HMAC Signing)

```bash
cargo run -p chadreview_relay_testing -- \
  --url http://localhost:8080 \
  --instance-id test-instance \
  --secret my-webhook-secret \
  issue-comment \
  --owner octocat \
  --repo hello-world \
  --pr 123 \
  --body "Signed webhook!"
```

## Features

- **Library API**: Programmatically generate and send mock webhooks
- **CLI Tool**: Command-line interface for manual testing
- **HMAC Signing**: Optional webhook signature generation for testing auth
- **Realistic Data**: Generates complete GitHub webhook payloads
- **Flexible**: Works with local or remote relay servers
