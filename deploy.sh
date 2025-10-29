#!/usr/bin/env bash
set -e

# ChadReview Relay Server - Fly.io Deployment Script
# Usage: ./deploy.sh [command]
# Commands: setup, deploy, logs, status, destroy

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

APP_NAME="chadreview-relay"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

# Check if flyctl is installed
check_flyctl() {
    if ! command -v flyctl &> /dev/null; then
        print_error "flyctl not found. Install it from: https://fly.io/docs/hands-on/install-flyctl/"
        exit 1
    fi
    print_success "flyctl found: $(flyctl version | head -1)"
}

# Setup: Create app and set secrets
setup() {
    print_info "Setting up Fly.io app: $APP_NAME"
    echo ""

    check_flyctl

    # Check if already logged in
    if ! flyctl auth whoami &> /dev/null; then
        print_warning "Not logged in to Fly.io"
        flyctl auth login
    fi

    print_success "Logged in as: $(flyctl auth whoami)"

    # Create app
    if flyctl apps list | grep -q "$APP_NAME"; then
        print_warning "App $APP_NAME already exists"
    else
        print_info "Creating app: $APP_NAME"
        flyctl apps create "$APP_NAME" || {
            print_error "Failed to create app. Try a different name in fly.toml"
            exit 1
        }
        print_success "App created: $APP_NAME"
    fi

    # Set webhook secret
    echo ""
    print_info "Setting GITHUB_WEBHOOK_SECRET"
    read -sp "Enter your GitHub webhook secret: " secret
    echo ""

    if [ -z "$secret" ]; then
        print_warning "No secret provided. You can set it later with:"
        echo "  flyctl secrets set GITHUB_WEBHOOK_SECRET=your_secret"
    else
        flyctl secrets set GITHUB_WEBHOOK_SECRET="$secret"
        print_success "Secret set successfully"
    fi

    echo ""
    print_success "Setup complete! Run: ./deploy.sh deploy"
}

# Deploy the app
deploy() {
    print_info "Deploying $APP_NAME to Fly.io"
    echo ""

    check_flyctl

    # Verify cargo build works
    print_info "Testing local build first..."
    if cargo build --release -p chadreview_relay_server; then
        print_success "Local build successful"
    else
        print_error "Local build failed. Fix errors before deploying."
        exit 1
    fi

    # Deploy
    echo ""
    print_info "Deploying to Fly.io (this may take 5-10 minutes)..."
    flyctl deploy

    echo ""
    print_success "Deployment complete!"
    print_info "Your app is live at: https://$APP_NAME.fly.dev"
    echo ""
    print_info "Test health endpoint: curl https://$APP_NAME.fly.dev/health"
    print_info "View logs: ./deploy.sh logs"
}

# View logs
logs() {
    check_flyctl
    flyctl logs
}

# Check status
status() {
    check_flyctl
    echo "=== App Status ==="
    flyctl status
    echo ""
    echo "=== Machine List ==="
    flyctl machine list
    echo ""
    echo "=== Health Checks ==="
    flyctl checks list
    echo ""
    echo "=== Recent Logs ==="
    flyctl logs --lines 20
}

# Destroy app
destroy() {
    print_warning "This will destroy the app and all resources!"
    read -p "Are you sure? (yes/no): " confirm

    if [ "$confirm" = "yes" ]; then
        check_flyctl
        flyctl apps destroy "$APP_NAME"
        print_success "App destroyed"
    else
        print_info "Cancelled"
    fi
}

# Show help
help() {
    cat << EOF
ChadReview Relay Server - Fly.io Deployment

Usage: ./deploy.sh [command]

Commands:
  setup      - Create app and configure secrets (first time only)
  deploy     - Build and deploy the relay server
  logs       - View real-time logs
  status     - Show app status, machines, and health checks
  destroy    - Delete the app and all resources
  help       - Show this help message

Examples:
  ./deploy.sh setup           # First time setup
  ./deploy.sh deploy          # Deploy after changes
  ./deploy.sh logs            # Monitor in real-time
  ./deploy.sh status          # Check if everything is working

URLs after deployment:
  Health:    https://$APP_NAME.fly.dev/health
  WebSocket: wss://$APP_NAME.fly.dev/ws/{instance_id}
  Webhook:   https://$APP_NAME.fly.dev/webhook/{instance_id}

Documentation:
  QUICK_START_FLY.md       - Quick start guide
  DEPLOYMENT.md            - Full deployment docs
  DEPLOYMENT_CHECKLIST.md  - Step-by-step checklist
EOF
}

# Main
case "${1:-help}" in
    setup)
        setup
        ;;
    deploy)
        deploy
        ;;
    logs)
        logs
        ;;
    status)
        status
        ;;
    destroy)
        destroy
        ;;
    help|--help|-h)
        help
        ;;
    *)
        print_error "Unknown command: $1"
        echo ""
        help
        exit 1
        ;;
esac
