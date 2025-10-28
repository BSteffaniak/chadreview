# chadreview_relay_client

WebSocket client library for connecting local ChadReview instances to the relay server.

## Features

- Automatic reconnection on connection loss
- Subscribe to pull request webhook events
- Persistent instance ID across restarts
- Thread-safe async API

## Usage

```rust
use chadreview_relay_client::RelayClient;
use chadreview_relay_models::PrKey;
use std::sync::Arc;

#[tokio::main]
async fn main() {
    let instance_id = RelayClient::get_or_create_instance_id();
    let client = RelayClient::connect("wss://relay.chadreview.com", instance_id)
        .await
        .unwrap();

    let pr_key = PrKey {
        owner: "owner".to_string(),
        repo: "repo".to_string(),
        number: 123,
    };

    client.subscribe(pr_key, Arc::new(|event| {
        println!("Received webhook event: {:?}", event);
    })).await.unwrap();
}
```

## Configuration

The instance ID is automatically generated and stored in:

- Linux: `~/.config/chadreview/instance_id`
- macOS: `~/Library/Application Support/chadreview/instance_id`
- Windows: `%APPDATA%\chadreview\instance_id`
