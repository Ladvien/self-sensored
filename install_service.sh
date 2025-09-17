#!/bin/bash

# Install systemd service for self-sensored API

echo "Installing self-sensored systemd service..."

# Stop existing process if running
pkill -f self-sensored || true

# Copy service file to systemd directory
sudo cp self-sensored.service /etc/systemd/system/

# Reload systemd to recognize new service
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable self-sensored

# Start the service
sudo systemctl start self-sensored

# Check status
sudo systemctl status self-sensored --no-pager

echo ""
echo "Service installed and started!"
echo ""
echo "Useful commands:"
echo "  sudo systemctl status self-sensored   # Check status"
echo "  sudo systemctl stop self-sensored     # Stop service"
echo "  sudo systemctl start self-sensored    # Start service"
echo "  sudo systemctl restart self-sensored  # Restart service"
echo "  sudo journalctl -u self-sensored -f   # View logs"
echo ""
echo "API endpoint: http://192.168.1.104:9876/api/v1/ingest"
echo "Health check: http://192.168.1.104:9876/health"
echo "API Key: test_auto_export_key_2024"