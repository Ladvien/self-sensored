#!/bin/bash

# Log rotation script for self-sensored health API
# This script rotates the server.log file when it gets too large

LOG_FILE="/home/ladvien/self-sensored/server.log"
MAX_SIZE_MB=10  # Rotate when log exceeds 10MB
BACKUP_COUNT=5  # Keep 5 backup files

# Check if log file exists
if [ ! -f "$LOG_FILE" ]; then
    echo "Log file $LOG_FILE not found"
    exit 0
fi

# Get file size in MB
FILE_SIZE_MB=$(du -m "$LOG_FILE" | cut -f1)

# Check if rotation is needed
if [ "$FILE_SIZE_MB" -gt "$MAX_SIZE_MB" ]; then
    echo "Log file size is ${FILE_SIZE_MB}MB, rotating..."
    
    # Remove oldest backup
    if [ -f "${LOG_FILE}.${BACKUP_COUNT}" ]; then
        rm "${LOG_FILE}.${BACKUP_COUNT}"
    fi
    
    # Shift existing backups
    for i in $(seq $((BACKUP_COUNT-1)) -1 1); do
        if [ -f "${LOG_FILE}.${i}" ]; then
            mv "${LOG_FILE}.${i}" "${LOG_FILE}.$((i+1))"
        fi
    done
    
    # Move current log to .1
    mv "$LOG_FILE" "${LOG_FILE}.1"
    
    # Create new empty log file
    touch "$LOG_FILE"
    
    echo "Log rotation completed"
else
    echo "Log file size is ${FILE_SIZE_MB}MB, no rotation needed"
fi