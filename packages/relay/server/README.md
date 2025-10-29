# ChadReview Relay Server

Real-time webhook relay server for ChadReview. Receives GitHub webhooks and broadcasts them to connected ChadReview clients via WebSocket.

## Architecture

- **WebSocket endpoint**: `/ws/{instance_id}` - Clients connect and subscribe to PR updates
- **Webhook endpoint**: `/webhook/{instance_id}` - Receives GitHub webhooks
- **Health check**: `/health` - Returns "OK" for monitoring

## Local Development

```bash
# Run the server
cargo run -p chadreview_relay_server

# Run with environment variables
PORT=3001 GITHUB_WEBHOOK_SECRET=your_secret cargo run -p chadreview_relay_server

# Run tests
cargo test -p chadreview_relay_server
```

## Deployment to Fly.io

### Prerequisites

1. Install Fly.io CLI: https://fly.io/docs/hands-on/install-flyctl/
2. Create account: `flyctl auth signup`
3. Login: `flyctl auth login`

### Initial Setup

From the repository root:

```bash
# Create the Fly.io app
flyctl apps create chadreview-relay

# Set the webhook secret (REQUIRED)
flyctl secrets set GITHUB_WEBHOOK_SECRET=your_github_webhook_secret_here

# Deploy
flyctl deploy

# Check status
flyctl status

# View logs
flyctl logs

# Open app dashboard
flyctl dashboard
```

### Configuration

The app is configured via `fly.toml` in the repository root:

- **Auto-scaling**: Scales to zero when idle (saves costs)
- **Region**: `iad` (Northern Virginia) - change in `fly.toml` if needed
- **Memory**: 256MB (sufficient for relay server)
- **Health checks**: Uses `/health` endpoint

### Environment Variables

Set via `flyctl secrets`:

```bash
# Required
flyctl secrets set GITHUB_WEBHOOK_SECRET=your_secret

# Optional (defaults shown)
flyctl secrets set PORT=8080
flyctl secrets set HOST=0.0.0.0
```

### Monitoring

```bash
# View real-time logs
flyctl logs

# SSH into the machine
flyctl ssh console

# Check machine status
flyctl machine list

# View metrics
flyctl dashboard
```

### Updating

```bash
# Deploy new version
flyctl deploy

# Restart machines
flyctl machine restart
```

### Costs

With the free tier and auto-scaling to zero:

- **Idle**: $0/month (machine stops after 5 minutes of inactivity)
- **Active**: Pay only for seconds the machine is running
- **Free tier**: 3 shared-cpu VMs included, 160GB transfer/month

### URLs

After deployment:

- **WebSocket**: `wss://chadreview-relay.fly.dev/ws/{instance_id}`
- **Webhook**: `https://chadreview-relay.fly.dev/webhook/{instance_id}`
- **Health**: `https://chadreview-relay.fly.dev/health`

Configure GitHub webhook to point to: `https://chadreview-relay.fly.dev/webhook/{instance_id}`

### Troubleshooting

```bash
# Check if app is running
flyctl status

# View logs
flyctl logs

# Check machine health
flyctl checks list

# SSH into machine for debugging
flyctl ssh console

# Force restart
flyctl machine restart
```

### Regions

To change region, update `primary_region` in `fly.toml`:

- `iad` - Northern Virginia
- `ord` - Chicago
- `sjc` - San Jose
- `lhr` - London
- See all: https://fly.io/docs/reference/regions/

## Production Considerations

1. **Set webhook secret**: Always set `GITHUB_WEBHOOK_SECRET` in production
2. **Monitor logs**: Use `flyctl logs` or integrate with logging service
3. **Health checks**: Already configured in `fly.toml`
4. **Scaling**: Adjust `min_machines_running` in `fly.toml` if you need instant response (costs ~$2/month for 24/7 uptime)
5. **Multiple regions**: Add more machines in different regions for redundancy

## API Reference

### WebSocket Protocol

**Connect:**

```
wss://chadreview-relay.fly.dev/ws/{instance_id}
```

**Client Messages:**

```json
{"Subscribe": {"pr_key": {"owner": "org", "repo": "name", "number": 123}}}
{"Unsubscribe": {"pr_key": {"owner": "org", "repo": "name", "number": 123}}}
{"Ping": null}
```

**Server Messages:**

```json
{"Subscribed": {"pr_key": {"owner": "org", "repo": "name", "number": 123}}}
{"Unsubscribed": {"pr_key": {"owner": "org", "repo": "name", "number": 123}}}
{"Pong": null}
{"Webhook": {"instance_id": "...", "pr_key": {...}, "event": {...}}}
```

### Webhook Endpoint

**POST** `https://chadreview-relay.fly.dev/webhook/{instance_id}`

Headers:

- `X-Hub-Signature-256`: GitHub webhook signature
- `X-GitHub-Event`: Event type (pull_request, issue_comment, etc.)

Body: GitHub webhook payload (JSON)

## GitHub Webhook Configuration

When setting up webhooks in your GitHub repository:

1. **Payload URL**: `https://chadreview-relay.fly.dev/webhook/{instance_id}`
2. **Content type**: `application/json`
3. **Secret**: Same as `GITHUB_WEBHOOK_SECRET` environment variable
4. **Events**: Select:
    - Issue comments
    - Pull request review comments
    - Pull requests
