#!/bin/bash
set -e

echo "=== RustySquid Router Deployment Script ==="
echo

# Configuration
ROUTER_IP="192.168.50.1"
ROUTER_USER="noah"
BINARY_PATH="target/aarch64-unknown-linux-musl/release/rustysquid"

# Build release
echo "1. Building release binary for router (aarch64)..."
cargo build --release --target aarch64-unknown-linux-musl

# Check binary exists
if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

# Check binary size
SIZE=$(ls -lh "$BINARY_PATH" | awk '{print $5}')
echo "   Binary size: $SIZE"

# Deploy to router
echo
echo "2. Deploying to router at $ROUTER_IP..."
scp "$BINARY_PATH" "$ROUTER_USER@$ROUTER_IP:/tmp/rustysquid"

echo
echo "3. Setting up on router..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
# Make binary executable
chmod +x /tmp/rustysquid

# Stop existing instance if running
if [ -f /tmp/rustysquid.pid ]; then
    OLD_PID=$(cat /tmp/rustysquid.pid)
    if kill -0 $OLD_PID 2>/dev/null; then
        echo "   Stopping existing instance (PID $OLD_PID)..."
        kill $OLD_PID
        sleep 2
    fi
fi

# Start new instance
echo "   Starting RustySquid..."
export RUST_LOG=rustysquid=info
nohup /tmp/rustysquid > /tmp/rustysquid.log 2>&1 &
echo $! > /tmp/rustysquid.pid

# Wait for startup
sleep 2

# Check if running
PID=$(cat /tmp/rustysquid.pid)
if kill -0 $PID 2>/dev/null; then
    echo "   ✓ RustySquid started successfully (PID $PID)"
    echo "   Listening on port 3128"
else
    echo "   ✗ Failed to start RustySquid"
    tail -20 /tmp/rustysquid.log
    exit 1
fi
EOF

echo
echo "4. Verifying deployment..."
# Test the proxy
curl -x "$ROUTER_IP:3128" -I http://example.com 2>/dev/null | head -1

echo
echo "=== Deployment Complete ==="
echo "Proxy URL: http://$ROUTER_IP:3128"
echo "Logs: ssh $ROUTER_USER@$ROUTER_IP 'tail -f /tmp/rustysquid.log'"