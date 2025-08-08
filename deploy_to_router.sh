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

# Deploy init script if it exists
if [ -f "init.d_rustysquid" ]; then
    echo "   Deploying init script..."
    scp "init.d_rustysquid" "$ROUTER_USER@$ROUTER_IP:/tmp/rustysquid.init"
fi

echo
echo "3. Setting up on router..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
# Make binary executable
chmod +x /tmp/rustysquid

# Install init script if provided
if [ -f /tmp/rustysquid.init ]; then
    echo "   Installing init script..."
    cp /tmp/rustysquid.init /etc/init.d/rustysquid
    chmod +x /etc/init.d/rustysquid
    /etc/init.d/rustysquid enable
    echo "   ✓ Service enabled for automatic start on boot"
fi

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
# Configure iptables for transparent proxy
echo "   Setting up transparent proxy rules..."

# Remove any existing rules first
iptables -t nat -D PREROUTING -i br0 -p tcp --dport 80 -j REDIRECT --to-port 3128 2>/dev/null || true
iptables -t nat -D PREROUTING -i br0 -p tcp --dport 443 -j REDIRECT --to-port 3128 2>/dev/null || true

# Add transparent proxy rule for HTTP traffic ONLY
# WARNING: NEVER intercept HTTPS (port 443) - it breaks SSL/TLS connections!
iptables -t nat -A PREROUTING -i br0 -p tcp --dport 80 -j REDIRECT --to-port 3128
# HTTPS traffic passes through normally - no interception

# Allow traffic to the proxy port
iptables -D INPUT -p tcp --dport 3128 -j ACCEPT 2>/dev/null || true
iptables -A INPUT -p tcp --dport 3128 -j ACCEPT

# Save iptables rules (OpenWrt specific)
/etc/init.d/firewall reload 2>/dev/null || true

echo "   ✓ Transparent proxy rules configured"
EOF

echo
echo "4. Verifying deployment..."
# Test the proxy
curl -x "$ROUTER_IP:3128" -I http://example.com 2>/dev/null | head -1

# Verify iptables rules are active
echo
echo "5. Checking proxy integration..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
echo "   Active proxy rules:"
iptables -t nat -L PREROUTING -n | grep 3128 | head -3
echo
echo "   Active connections:"
netstat -tn | grep :3128 | head -5 || echo "   No active connections yet"
EOF

echo
echo "=== Deployment Complete ==="
echo "Proxy URL: http://$ROUTER_IP:3128"
echo "Transparent proxy: Active for HTTP traffic only (HTTPS passes through)"
echo "Logs: ssh $ROUTER_USER@$ROUTER_IP 'tail -f /tmp/rustysquid.log'"