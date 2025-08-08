#!/bin/bash
# Safe, predictable deployment following Toyota Way principles
# No surprises, no boobytraps, just reliable deployment

set -e  # Stop on any error

# Color codes for clear visual feedback
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration - single source of truth
ROUTER_IP="${ROUTER_IP:-192.168.50.1}"
ROUTER_USER="${ROUTER_USER:-noah}"
BINARY_NAME="rustysquid"
DEPLOY_PATH="/tmp/${BINARY_NAME}"
LOG_PATH="/tmp/${BINARY_NAME}.log"
PID_PATH="/tmp/${BINARY_NAME}.pid"
PROXY_PORT="3128"

# Poka-yoke: Verify prerequisites
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🔍 Pre-deployment Safety Checks"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Check 1: Binary exists
if [ ! -f "target/aarch64-unknown-linux-musl/release/${BINARY_NAME}" ]; then
    echo -e "${RED}✗ Binary not found. Run 'make build' first${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Binary exists${NC}"

# Check 2: Router connectivity
if ! ssh -o ConnectTimeout=5 "${ROUTER_USER}@${ROUTER_IP}" "echo 'connected'" > /dev/null 2>&1; then
    echo -e "${RED}✗ Cannot connect to router at ${ROUTER_IP}${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Router reachable${NC}"

# Check 3: Show current state (transparency)
echo
echo "📊 Current State on Router:"
ssh "${ROUTER_USER}@${ROUTER_IP}" << EOF
echo "  Process: \$(ps w | grep ${BINARY_NAME} | grep -v grep | wc -l) instance(s) running"
echo "  Port ${PROXY_PORT}: \$(netstat -tln | grep -c :${PROXY_PORT} || echo 0) listener(s)"
echo "  Iptables rules: \$(iptables -t nat -L PREROUTING -n | grep -c ${PROXY_PORT} || echo 0) rule(s)"
EOF

# Confirmation gate
echo
echo -e "${YELLOW}📋 Deployment Plan:${NC}"
echo "  1. Stop existing proxy gracefully"
echo "  2. Deploy new binary to ${DEPLOY_PATH}"
echo "  3. Start proxy service"
echo "  4. Configure transparent proxy rules"
echo "  5. Verify everything works"
echo

# Skip confirmation if SKIP_CONFIRM is set (for automation)
if [ -z "$SKIP_CONFIRM" ]; then
    read -p "Proceed with deployment? (y/n) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        echo "Deployment cancelled"
        exit 0
    fi
else
    echo "Auto-confirming deployment (SKIP_CONFIRM=1)"
fi

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "🚀 Starting Safe Deployment"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

# Step 1: Stop existing service gracefully
echo "1️⃣  Stopping existing service..."
ssh "${ROUTER_USER}@${ROUTER_IP}" << EOF
if [ -f "${PID_PATH}" ]; then
    OLD_PID=\$(cat ${PID_PATH})
    if kill -0 \$OLD_PID 2>/dev/null; then
        kill -TERM \$OLD_PID
        sleep 2
        echo "   Stopped PID \$OLD_PID gracefully"
    fi
fi
# Clean stop of any orphaned processes
pkill -f ${BINARY_NAME} 2>/dev/null || true
EOF

# Step 2: Deploy binary
echo "2️⃣  Deploying binary..."
scp -q "target/aarch64-unknown-linux-musl/release/${BINARY_NAME}" \
    "${ROUTER_USER}@${ROUTER_IP}:${DEPLOY_PATH}"
ssh "${ROUTER_USER}@${ROUTER_IP}" "chmod +x ${DEPLOY_PATH}"
echo "   Binary deployed to ${DEPLOY_PATH}"

# Step 3: Start service
echo "3️⃣  Starting service..."
ssh "${ROUTER_USER}@${ROUTER_IP}" << EOF
export RUST_LOG=rustysquid=info
nohup ${DEPLOY_PATH} > ${LOG_PATH} 2>&1 &
echo \$! > ${PID_PATH}
sleep 2

# Verify it started
PID=\$(cat ${PID_PATH})
if kill -0 \$PID 2>/dev/null; then
    echo "   Started successfully (PID \$PID)"
else
    echo "   ERROR: Failed to start"
    tail -5 ${LOG_PATH}
    exit 1
fi
EOF

# Step 4: Configure transparent proxy (idempotent)
echo "4️⃣  Configuring transparent proxy..."
ssh "${ROUTER_USER}@${ROUTER_IP}" << EOF
# Remove old rules (idempotent)
iptables -t nat -D PREROUTING -i br-lan -p tcp --dport 80 -j REDIRECT --to-port ${PROXY_PORT} 2>/dev/null || true
iptables -t nat -D PREROUTING -i br-lan -p tcp --dport 443 -j REDIRECT --to-port ${PROXY_PORT} 2>/dev/null || true

# Add new rules
iptables -t nat -A PREROUTING -i br-lan -p tcp --dport 80 -j REDIRECT --to-port ${PROXY_PORT}
iptables -t nat -A PREROUTING -i br-lan -p tcp --dport 443 -j REDIRECT --to-port ${PROXY_PORT}

# Allow proxy port
iptables -D INPUT -p tcp --dport ${PROXY_PORT} -j ACCEPT 2>/dev/null || true
iptables -A INPUT -p tcp --dport ${PROXY_PORT} -j ACCEPT

echo "   Transparent proxy rules configured"
EOF

# Step 5: Verification
echo "5️⃣  Verifying deployment..."
echo

# Test 1: Process running
if ssh "${ROUTER_USER}@${ROUTER_IP}" "kill -0 \$(cat ${PID_PATH}) 2>/dev/null"; then
    echo -e "${GREEN}✓ Process running${NC}"
else
    echo -e "${RED}✗ Process not running${NC}"
    exit 1
fi

# Test 2: Port listening
if ssh "${ROUTER_USER}@${ROUTER_IP}" "netstat -tln | grep -q :${PROXY_PORT}"; then
    echo -e "${GREEN}✓ Port ${PROXY_PORT} listening${NC}"
else
    echo -e "${RED}✗ Port ${PROXY_PORT} not listening${NC}"
    exit 1
fi

# Test 3: Iptables rules
RULE_COUNT=$(ssh "${ROUTER_USER}@${ROUTER_IP}" "iptables -t nat -L PREROUTING -n | grep -c ${PROXY_PORT}" || echo 0)
if [ "$RULE_COUNT" -ge 2 ]; then
    echo -e "${GREEN}✓ Transparent proxy rules active (${RULE_COUNT} rules)${NC}"
else
    echo -e "${YELLOW}⚠ Only ${RULE_COUNT} proxy rules found${NC}"
fi

# Test 4: Cache functionality
echo "Testing cache..."
ssh "${ROUTER_USER}@${ROUTER_IP}" << 'EOF' > /dev/null 2>&1
# Make a test request
echo -e "GET http://example.com/test HTTP/1.0\r\nHost: example.com\r\n\r\n" | nc localhost 3128 > /dev/null 2>&1
EOF

if ssh "${ROUTER_USER}@${ROUTER_IP}" "tail -5 ${LOG_PATH} | grep -q CACHE"; then
    echo -e "${GREEN}✓ Cache functionality confirmed${NC}"
else
    echo -e "${YELLOW}⚠ Cache logs not yet visible${NC}"
fi

echo
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "${GREEN}✅ Deployment Complete - No Surprises!${NC}"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo
echo "📋 Summary:"
echo "  • Proxy: http://${ROUTER_IP}:${PROXY_PORT}"
echo "  • Logs: ssh ${ROUTER_USER}@${ROUTER_IP} 'tail -f ${LOG_PATH}'"
echo "  • Status: ssh ${ROUTER_USER}@${ROUTER_IP} 'ps w | grep ${BINARY_NAME}'"
echo
echo "🔧 Rollback if needed:"
echo "  ssh ${ROUTER_USER}@${ROUTER_IP} 'pkill -f ${BINARY_NAME}'"
echo "  ssh ${ROUTER_USER}@${ROUTER_IP} 'iptables -t nat -F PREROUTING'"