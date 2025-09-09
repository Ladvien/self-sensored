#!/bin/bash

# Simple log truncation script for self-sensored health API
LOG_FILE="/home/ladvien/self-sensored/server.log"

if [ -f "$LOG_FILE" ]; then
    echo "Truncating server logs..."
    > "$LOG_FILE"
    echo "Server logs truncated successfully"
else
    echo "Log file not found: $LOG_FILE"
fi