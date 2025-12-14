#!/bin/bash
#
# Stop ZyberLink Server Stack
# Stops all containers started by docker-compose.localnet.yml
#

set -e

PROJECT_ROOT="$(cd "$(dirname "$0")/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_step() {
    echo -e "\n${BLUE}===${NC} $1 ${BLUE}===${NC}\n"
}

cd "$PROJECT_ROOT"

log_step "Stopping ZyberLink Server Stack"

COMPOSE_FILE="infra/docker/docker-compose.localnet.yml"

if [ ! -f "$COMPOSE_FILE" ]; then
    log_warn "Compose file not found: $COMPOSE_FILE"
    log_warn "Attempting to stop containers by name..."

    # Fallback: stop containers by name
    log_info "Stopping containers..."
    podman stop zyberlink-nginx 2>/dev/null || true
    podman stop zyberlink-public-api 2>/dev/null || true
    podman stop zyberlink-x402 2>/dev/null || true
    podman stop zyberlink-blink 2>/dev/null || true
    podman stop zyberlink-webapp 2>/dev/null || true
    podman stop zyberlink-postgres 2>/dev/null || true

    log_info "Stopped. Cleanup with: podman system prune"
    exit 0
fi

# Use podman-compose to stop
log_info "Stopping services..."
podman-compose -f "$COMPOSE_FILE" down

log_info "✓ All services stopped"

# Optionally show remaining containers
REMAINING=$(podman ps --filter "name=zyberlink" --format "{{.Names}}" 2>/dev/null || true)
if [ -n "$REMAINING" ]; then
    log_warn "Some ZyberLink containers are still running:"
    echo "$REMAINING"
    echo ""
    echo "To force stop:"
    echo "  podman stop $REMAINING"
fi

log_step "Server Stack Stopped"
echo " To start again:"
echo "  ./scripts/start-servers.sh"
echo ""
echo " To clean up volumes and images:"
echo "  podman-compose -f $COMPOSE_FILE down -v"
echo "  podman system prune -a"
echo ""
