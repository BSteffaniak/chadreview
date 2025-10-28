# chadreview_relay_models

Shared webhook event models for the ChadReview relay system.

This package contains the data structures used for communication between:

- GitHub webhooks
- Relay server
- Relay clients (local ChadReview instances)

## Models

- `WebhookEvent`: GitHub webhook events (issue_comment, pull_request_review_comment, pull_request)
- `RelayMessage`: Messages sent from relay server to clients
- `ClientMessage`: Messages sent from clients to relay server
- `ServerMessage`: Messages sent from relay server to clients
- `PrKey`: Unique identifier for a pull request (owner/repo/number)

## Usage

```rust
use chadreview_relay_models::{WebhookEvent, PrKey};
```
