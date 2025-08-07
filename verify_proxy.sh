#!/bin/bash
# RustySquid Proxy Verification Script

set -e

ROUTER_IP="192.168.50.1"
ROUTER_USER="noah"

echo "=== RustySquid Proxy Verification ==="
echo

# Check if proxy is running
echo "1. Checking proxy process..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
if ps w | grep -v grep | grep -q rustysquid; then
    PID=$(ps w | grep -v grep | grep rustysquid | awk '{print $1}')
    echo "   ✓ RustySquid is running (PID: $PID)"
else
    echo "   ✗ RustySquid is NOT running"
    exit 1
fi
EOF

# Check if proxy is listening
echo
echo "2. Checking proxy port..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
if netstat -tunl | grep -q ":3128"; then
    echo "   ✓ Proxy is listening on port 3128"
else
    echo "   ✗ Proxy is NOT listening on port 3128"
    exit 1
fi
EOF

# Check iptables rules
echo
echo "3. Checking transparent proxy rules..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
HTTP_RULE=$(iptables -t nat -L PREROUTING -n | grep -c "tcp dpt:80 redir ports 3128" || echo 0)
HTTPS_RULE=$(iptables -t nat -L PREROUTING -n | grep -c "tcp dpt:443 redir ports 3128" || echo 0)

if [ "$HTTP_RULE" -gt 0 ]; then
    echo "   ✓ HTTP traffic redirection rule is active"
else
    echo "   ✗ HTTP traffic redirection rule is NOT active"
fi

if [ "$HTTPS_RULE" -gt 0 ]; then
    echo "   ✓ HTTPS traffic redirection rule is active"
else
    echo "   ✗ HTTPS traffic redirection rule is NOT active"
fi

if [ "$HTTP_RULE" -eq 0 ] || [ "$HTTPS_RULE" -eq 0 ]; then
    exit 1
fi
EOF

# Check service status
echo
echo "4. Checking service configuration..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
if [ -f /etc/init.d/rustysquid ]; then
    echo "   ✓ Init script is installed"
    if /etc/init.d/rustysquid enabled 2>/dev/null; then
        echo "   ✓ Service is enabled for auto-start"
    else
        echo "   ⚠ Service auto-start status unknown"
    fi
else
    echo "   ⚠ Init script not installed (manual start required)"
fi
EOF

# Test proxy functionality
echo
echo "5. Testing proxy functionality..."
if curl -s -x "$ROUTER_IP:3128" -I http://example.com 2>/dev/null | grep -q "200 OK\|301\|302"; then
    echo "   ✓ Proxy is responding to HTTP requests"
else
    echo "   ✗ Proxy is NOT responding to HTTP requests"
    exit 1
fi

# Check active connections
echo
echo "6. Checking active connections..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
CONNECTIONS=$(netstat -tn 2>/dev/null | grep -c ":3128" || echo 0)
echo "   Active proxy connections: $CONNECTIONS"
EOF

# Check logs for errors
echo
echo "7. Checking recent logs..."
ssh "$ROUTER_USER@$ROUTER_IP" << 'EOF'
if [ -f /tmp/rustysquid.log ]; then
    ERROR_COUNT=$(tail -100 /tmp/rustysquid.log | grep -ci "error" || echo 0)
    if [ "$ERROR_COUNT" -gt 0 ]; then
        echo "   ⚠ Found $ERROR_COUNT error(s) in recent logs"
        echo "   Recent errors:"
        tail -100 /tmp/rustysquid.log | grep -i "error" | tail -3
    else
        echo "   ✓ No errors in recent logs"
    fi
else
    echo "   ⚠ Log file not found"
fi
EOF

echo
echo "=== Verification Complete ==="
echo
echo "Summary:"
echo "- Proxy URL: http://$ROUTER_IP:3128"
echo "- Transparent proxy: Active for HTTP/HTTPS from LAN"
echo "- View logs: ssh $ROUTER_USER@$ROUTER_IP 'tail -f /tmp/rustysquid.log'"
echo "- Restart service: ssh $ROUTER_USER@$ROUTER_IP '/etc/init.d/rustysquid restart'"