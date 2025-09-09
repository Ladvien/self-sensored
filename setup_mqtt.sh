#!/bin/bash

# MQTT Setup Script for 192.168.1.110
# Run with: sudo bash setup_mqtt.sh

set -e

echo "=== MQTT Setup for Health Export API ==="
echo "Starting at $(date)"

# Step 1: Install Mosquitto
echo ""
echo "Step 1: Installing Mosquitto MQTT Broker..."
apt update
apt install -y mosquitto mosquitto-clients

# Step 2: Stop Mosquitto to configure
echo ""
echo "Step 2: Stopping Mosquitto for configuration..."
systemctl stop mosquitto || true

# Step 3: Backup existing config if it exists
echo ""
echo "Step 3: Backing up existing configuration..."
if [ -f /etc/mosquitto/mosquitto.conf ]; then
    cp /etc/mosquitto/mosquitto.conf /etc/mosquitto/mosquitto.conf.backup.$(date +%Y%m%d_%H%M%S)
    echo "Backed up existing config"
fi

# Step 4: Copy new configuration
echo ""
echo "Step 4: Installing new Mosquitto configuration..."
cp /home/ladvien/self-sensored/mosquitto.conf /etc/mosquitto/mosquitto.conf
echo "Configuration installed"

# Step 5: Create password file with health_export user
echo ""
echo "Step 5: Creating MQTT user credentials..."
echo "Creating user 'health_export' with password 'HealthData2024!'"
mosquitto_passwd -b -c /etc/mosquitto/passwd health_export 'HealthData2024!'

# Step 6: Set proper permissions
echo ""
echo "Step 6: Setting file permissions..."
chown mosquitto:mosquitto /etc/mosquitto/passwd
chmod 600 /etc/mosquitto/passwd
touch /var/log/mosquitto/mosquitto.log
chown mosquitto:mosquitto /var/log/mosquitto/mosquitto.log

# Step 7: Configure firewall
echo ""
echo "Step 7: Configuring firewall..."
# Check if ufw is active
if command -v ufw &> /dev/null; then
    if ufw status | grep -q "Status: active"; then
        echo "Adding UFW rule for MQTT WebSocket from 192.168.1.102..."
        ufw allow from 192.168.1.102 to any port 9001
    else
        echo "UFW is installed but not active"
    fi
else
    echo "UFW not found, using iptables..."
    iptables -A INPUT -p tcp -s 192.168.1.102 --dport 9001 -j ACCEPT
    # Save iptables rules
    if command -v iptables-save &> /dev/null; then
        iptables-save > /etc/iptables/rules.v4
    fi
fi

# Step 8: Enable and start Mosquitto
echo ""
echo "Step 8: Starting Mosquitto service..."
systemctl enable mosquitto
systemctl start mosquitto

# Step 9: Verify installation
echo ""
echo "Step 9: Verifying installation..."
sleep 2

# Check if service is running
if systemctl is-active --quiet mosquitto; then
    echo "✓ Mosquitto service is running"
else
    echo "✗ Mosquitto service failed to start"
    systemctl status mosquitto
    exit 1
fi

# Check if listening on port 9001
if netstat -tlnp 2>/dev/null | grep -q ":9001"; then
    echo "✓ Mosquitto is listening on port 9001 (WebSocket)"
else
    echo "✗ Port 9001 is not listening"
    echo "Checking Mosquitto logs:"
    tail -n 20 /var/log/mosquitto/mosquitto.log
    exit 1
fi

# Check if listening on port 1883
if netstat -tlnp 2>/dev/null | grep -q "127.0.0.1:1883"; then
    echo "✓ Mosquitto is listening on port 1883 (localhost only)"
else
    echo "⚠ Port 1883 is not listening (this is okay if not needed)"
fi

echo ""
echo "=== MQTT Setup Complete ==="
echo ""
echo "MQTT Broker is configured with:"
echo "  - WebSocket on port 9001 (all interfaces)"
echo "  - Standard MQTT on port 1883 (localhost only)"
echo "  - Username: health_export"
echo "  - Password: HealthData2024!"
echo ""
echo "Test commands:"
echo "  Local test:    mosquitto_sub -h localhost -p 1883 -u health_export -P 'HealthData2024!' -t test/# -v"
echo "  WebSocket test: mosquitto_sub -h localhost -p 9001 -u health_export -P 'HealthData2024!' -t test/# -v"
echo ""
echo "From 192.168.1.102, test with:"
echo "  curl http://192.168.1.110:9001"
echo ""
echo "Logs available at: /var/log/mosquitto/mosquitto.log"
echo "Service status: systemctl status mosquitto"