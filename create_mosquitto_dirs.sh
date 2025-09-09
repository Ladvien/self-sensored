#!/bin/bash

echo "Creating missing Mosquitto directories..."
mkdir -p /var/lib/mosquitto
mkdir -p /var/run/mosquitto

echo "Setting ownership..."
chown mosquitto:mosquitto /var/lib/mosquitto
chown mosquitto:mosquitto /var/run/mosquitto

echo "Copying Manjaro-specific configuration..."
cp /home/ladvien/self-sensored/mosquitto_manjaro.conf /etc/mosquitto/mosquitto.conf

echo "Restarting Mosquitto..."
systemctl restart mosquitto

echo "Checking status..."
sleep 2
systemctl is-active mosquitto && echo "✓ Mosquitto is running" || echo "✗ Still failed"

echo "Checking ports..."
ss -tln | grep :9001 && echo "✓ Port 9001 listening" || echo "✗ Port 9001 not listening"