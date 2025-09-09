#!/bin/bash

echo "=== Simple Mosquitto Fix ==="

# Copy working config
cp /home/ladvien/self-sensored/mosquitto_working.conf /etc/mosquitto/mosquitto.conf

# Restart mosquitto
systemctl restart mosquitto

sleep 2

# Check status
if systemctl is-active mosquitto >/dev/null 2>&1; then
    echo "✓ Mosquitto is running"
    ss -tln | grep :9001 && echo "✓ Port 9001 listening" || echo "✗ Port 9001 not listening"
    ss -tln | grep :1883 && echo "✓ Port 1883 listening" || echo "✗ Port 1883 not listening"
else
    echo "✗ Mosquitto failed to start"
    systemctl status mosquitto --no-pager -l
fi