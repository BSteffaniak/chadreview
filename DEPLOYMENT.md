# Deployment Guide

Quick reference for deploying ChadReview Relay Server to Fly.io.

## Quick Start

```bash
# 1. Login to Fly.io (first time only)
flyctl auth login

# 2. Create the app (first time only)
flyctl apps create chadreview-relay

# 3. Set your GitHub webhook secret
flyctl secrets set GITHUB_WEBHOOK_SECRET=your_github_webhook_secret_here

# 4. Deploy
flyctl deploy

# 5. Verify it's running
flyctl status
curl https://chadreview-relay.fly.dev/health
```

## Your App URLs

After deployment, your relay server will be available at:

- **Health check**: `https://chadreview-relay.fly.dev/health`
- **WebSocket**: `wss://chadreview-relay.fly.dev/ws/{instance_id}`
- **Webhook**: `https://chadreview-relay.fly.dev/webhook/{instance_id}`

## GitHub Webhook Setup

1. Go to your GitHub repository → Settings → Webhooks → Add webhook
2. **Payload URL**: `https://chadreview-relay.fly.dev/webhook/{instance_id}`
    - Replace `{instance_id}` with a unique identifier (e.g., `my-instance`)
3. **Content type**: `application/json`
4. **Secret**: Use the same value you set for `GITHUB_WEBHOOK_SECRET`
5. **Events**: Select individual events:
    - ✅ Issue comments
    - ✅ Pull request review comments
    - ✅ Pull requests
6. Click "Add webhook"

## Cost Optimization

The relay server is configured to **scale to zero** when idle:

- **Idle (no connections)**: $0/month
- **Active (connections open)**: Pay only for runtime seconds
- **Free tier**: 3 shared-cpu VMs, 160GB transfer/month

To keep it always responsive (skip 5-10s cold start), edit `fly.toml`:

```toml
[http_service]
  min_machines_running = 1  # Change from 0 to 1
```

Cost: ~$2/month for 24/7 uptime.

## Monitoring

```bash
# View logs in real-time
flyctl logs

# Check app status
flyctl status

# View dashboard
flyctl dashboard

# Check health
curl https://chadreview-relay.fly.dev/health
```

## Updating

When you make changes to the relay server code:

```bash
# Deploy the new version
flyctl deploy

# Or deploy and tail logs
flyctl deploy --detach && flyctl logs
```

## Troubleshooting

### App won't start

```bash
# Check logs
flyctl logs

# Check if build succeeded
flyctl status

# Try restarting
flyctl machine restart
```

### WebSocket connections failing

```bash
# Verify health endpoint works
curl https://chadreview-relay.fly.dev/health

# Check if app is running
flyctl status

# View logs for connection attempts
flyctl logs
```

### Webhooks not being received

1. Check GitHub webhook delivery status (Settings → Webhooks → Recent Deliveries)
2. Verify the webhook URL is correct
3. Check logs: `flyctl logs`
4. Verify webhook secret matches: `flyctl secrets list`

## Advanced

### Multiple Regions

To deploy in multiple regions for lower latency:

```bash
# Add a machine in Europe
flyctl machine clone --region lhr

# Add a machine in Asia
flyctl machine clone --region nrt
```

### Custom Domain

```bash
# Add a custom domain
flyctl certs create relay.yourdomain.com

# Update DNS with the provided records
```

### Scaling

```bash
# List machines
flyctl machine list

# Clone for more capacity
flyctl machine clone

# Remove a machine
flyctl machine destroy <machine-id>
```

## See Also

- [Fly.io Docs](https://fly.io/docs/)
- [Relay Server README](packages/relay/server/README.md)
- [Fly.io Pricing](https://fly.io/docs/about/pricing/)
