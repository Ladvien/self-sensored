#!/bin/bash

echo "=== Fixing Mosquitto Setup on Manjaro ==="

# Create required directories
echo "Creating required directories..."
sudo mkdir -p /var/lib/mosquitto
sudo mkdir -p /var/log/mosquitto
sudo mkdir -p /var/run/mosquitto

# Set proper ownership
echo "Setting proper ownership..."
sudo chown -R mosquitto:mosquitto /var/lib/mosquitto
sudo chown -R mosquitto:mosquitto /var/log/mosquitto
sudo chown -R mosquitto:mosquitto /var/run/mosquitto

# Copy the Manjaro-specific configuration
echo "Installing Manjaro-specific configuration..."
sudo cp /home/ladvien/self-sensored/mosquitto_manjaro.conf /etc/mosquitto/mosquitto.conf

# Ensure password file exists and has correct permissions
echo "Verifying password file..."
if [ ! -f /etc/mosquitto/passwd ]; then
    echo "Creating password file..."
    sudo mosquitto_passwd -b -c /etc/mosquitto/passwd health_export 'HealthData2024!'
fi
sudo chown mosquitto:mosquitto /etc/mosquitto/passwd
sudo chmod 600 /etc/mosquitto/passwd

# Test the configuration
echo "Testing configuration..."
sudo mosquitto -c /etc/mosquitto/mosquitto.conf -t

# Restart the service
echo "Restarting Mosquitto service..."
sudo systemctl daemon-reload
sudo systemctl restart mosquitto

# Check status
sleep 2
echo ""
echo "Checking service status..."
sudo systemctl is-active mosquitto && echo "✓ Mosquitto is running" || echo "✗ Mosquitto failed to start"

# Check if ports are listening
echo ""
echo "Checking ports..."
ss -tln | grep :1883 && echo "✓ Port 1883 is listening" || echo "✗ Port 1883 is not listening"
ss -tln | grep :9001 && echo "✓ Port 9001 is listening" || echo "✗ Port 9001 is not listening"

echo ""
echo "To view logs, run:"
echo "  sudo journalctl -u mosquitto -n 50"
echo "  sudo tail -f /var/log/mosquitto/mosquitto.log"