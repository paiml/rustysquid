#!/bin/sh
# Example configuration for ASUS routers

# Install rustysquid on router
echo "Installing RustySquid on ASUS router..."

# Copy binary to router
scp target/aarch64-unknown-linux-musl/release/rustysquid admin@192.168.1.1:/tmp/rustysquid

# SSH to router and setup
ssh admin@192.168.1.1 << 'EOF'
# Make binary executable
chmod +x /tmp/rustysquid

# Create startup script
cat > /jffs/scripts/rustysquid.sh << 'SCRIPT'
#!/bin/sh
# RustySquid startup script

# Set environment
export RUST_LOG=rustysquid=info

# Start proxy with memory limit
if [ -f /tmp/rustysquid ]; then
    # Limit to 50MB memory
    ulimit -m 51200
    ulimit -v 51200
    
    # Start in background
    /tmp/rustysquid > /tmp/rustysquid.log 2>&1 &
    echo $! > /tmp/rustysquid.pid
    
    logger "RustySquid started with PID $(cat /tmp/rustysquid.pid)"
fi
SCRIPT

# Make startup script executable
chmod +x /jffs/scripts/rustysquid.sh

# Add to services-start
echo "/jffs/scripts/rustysquid.sh" >> /jffs/scripts/services-start

# Configure firewall to redirect HTTP traffic through proxy
iptables -t nat -A PREROUTING -i br0 -p tcp --dport 80 -j REDIRECT --to-port 3128

# Start the service
/jffs/scripts/rustysquid.sh

# Check if running
if [ -f /tmp/rustysquid.pid ]; then
    PID=$(cat /tmp/rustysquid.pid)
    if kill -0 $PID 2>/dev/null; then
        echo "RustySquid is running with PID $PID"
    else
        echo "RustySquid failed to start"
    fi
fi
EOF

echo "Installation complete!"