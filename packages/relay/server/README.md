# chadreview_relay_server

Relay server for propagating GitHub webhook events to local ChadReview instances.

## Features

- Receives GitHub webhooks
- Maintains WebSocket connections to local instances
- Routes webhook events based on PR subscriptions
- GitHub webhook signature verification

## Running

```bash
# Set environment variables
export PORT=8080
export HOST=0.0.0.0
export GITHUB_WEBHOOK_SECRET=your_webhook_secret  # Optional but recommended

# Run the server
cargo run -p chadreview_relay_server
```

## Endpoints

- `POST /webhook/{instance_id}` - Receive GitHub webhooks
- `GET /ws/{instance_id}` - WebSocket connection for clients
- `GET /health` - Health check endpoint

## GitHub Webhook Configuration

When setting up webhooks in your GitHub repository:

1. **Payload URL**: `https://your-relay-server.com/webhook/{instance_id}`
2. **Content type**: `application/json`
3. **Secret**: Set `GITHUB_WEBHOOK_SECRET` environment variable
4. **Events**: Select:
    - Issue comments
    - Pull request review comments
    - Pull requests

## Deployment

The relay server can be deployed to any platform that supports Rust binaries:

- Fly.io
- Railway
- Heroku
- AWS (ECS, Lambda)
- DigitalOcean App Platform

Example Dockerfile:

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p chadreview_relay_server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/relay-server /usr/local/bin/
CMD ["relay-server"]
```
