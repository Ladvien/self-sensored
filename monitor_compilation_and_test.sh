#!/bin/bash

# Compilation Monitor and Auto-Test Executor
# Test Orchestrator Agent - Claude Code
# Created: 2025-09-14

set -e

# Color definitions
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_status() {
    local status=$1
    local message=$2
    case $status in
        "SUCCESS") echo -e "${GREEN} $message${NC}" ;;
        "ERROR") echo -e "${RED}L $message${NC}" ;;
        "WARNING") echo -e "${YELLOW}   $message${NC}" ;;
        "INFO") echo -e "${BLUE}9  $message${NC}" ;;
    esac
}

echo "= Compilation Monitor & Test Orchestrator"
echo "=========================================="
print_status "INFO" "Monitoring compilation status and waiting for completion..."

# Monitor compilation in a loop
LAST_ERROR_COUNT=999
CHECK_INTERVAL=30  # Check every 30 seconds
MAX_CHECKS=60      # Maximum 30 minutes of monitoring

for ((i=1; i<=MAX_CHECKS; i++)); do
    print_status "INFO" "Check $i/$MAX_CHECKS - Checking compilation status..."

    # Check compilation and capture error count
    if cargo check --quiet 2>/dev/null; then
        print_status "SUCCESS" "Compilation successful! All errors resolved."
        echo ""
        print_status "INFO" "Automatically launching comprehensive test suite..."

        # Run the comprehensive test suite
        if ./run_comprehensive_tests.sh; then
            print_status "SUCCESS" "All tests passed! System ready for deployment."
            exit 0
        else
            print_status "ERROR" "Some tests failed. Check test output above."
            exit 1
        fi
    else
        # Count current errors
        ERROR_OUTPUT=$(cargo check 2>&1)
        CURRENT_ERROR_COUNT=$(echo "$ERROR_OUTPUT" | grep -c "error:" 2>/dev/null || echo "0")

        # Show progress if error count changed
        if [ "$CURRENT_ERROR_COUNT" != "$LAST_ERROR_COUNT" ]; then
            if [ "$CURRENT_ERROR_COUNT" -lt "$LAST_ERROR_COUNT" ]; then
                print_status "SUCCESS" "Progress: $CURRENT_ERROR_COUNT errors (down from $LAST_ERROR_COUNT)"
            else
                print_status "WARNING" "Errors increased: $CURRENT_ERROR_COUNT (up from $LAST_ERROR_COUNT)"
            fi
            LAST_ERROR_COUNT=$CURRENT_ERROR_COUNT
        else
            print_status "INFO" "Still $CURRENT_ERROR_COUNT compilation errors remaining..."
        fi

        # Show most recent errors for context
        if [ "$CURRENT_ERROR_COUNT" -gt 0 ] && [ "$CURRENT_ERROR_COUNT" -le 5 ]; then
            echo "Recent errors:"
            echo "$ERROR_OUTPUT" | grep -A2 "error:" | head -10
        fi
    fi

    # Wait before next check
    if [ $i -lt $MAX_CHECKS ]; then
        print_status "INFO" "Waiting ${CHECK_INTERVAL}s before next check..."
        sleep $CHECK_INTERVAL
    fi
done

print_status "WARNING" "Reached maximum monitoring time (30 minutes)"
print_status "INFO" "Compilation is taking longer than expected. Check active agents."
exit 1