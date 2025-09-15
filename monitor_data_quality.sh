#!/bin/bash

# Data Processing Quality Monitor
# Runs continuous monitoring for health data validation and processing quality

REVIEW_FILE="/mnt/datadrive_m2/self-sensored/review_notes.md"
LAST_COMMIT_FILE="/tmp/last_monitored_commit"
CHECK_INTERVAL=30

# Initialize with current commit if first run
if [ ! -f "$LAST_COMMIT_FILE" ]; then
    git rev-parse HEAD > "$LAST_COMMIT_FILE"
    echo "Initialized monitoring from commit: $(cat $LAST_COMMIT_FILE)"
fi

monitor_commits() {
    local current_commit=$(git rev-parse HEAD)
    local last_monitored=$(cat "$LAST_COMMIT_FILE")

    if [ "$current_commit" != "$last_monitored" ]; then
        echo "New commit detected: $current_commit"
        echo "Analyzing data processing quality changes..."

        # Get commit details
        local commit_info=$(git log --oneline -1 $current_commit)
        local commit_short=$(echo $current_commit | cut -c1-7)
        local commit_message=$(git log --format="%s" -1 $current_commit)
        local commit_date=$(date '+%Y-%m-%d %H:%M:%S')

        # Check if commit affects data processing areas
        local changed_files=$(git diff --name-only $last_monitored $current_commit)
        local data_files=$(echo "$changed_files" | grep -E "(models|validation|batch|processing|handler)" || true)

        if [ -n "$data_files" ]; then
            echo "Data processing files changed:"
            echo "$data_files"

            # Analyze impact based on changed files
            analyze_data_impact "$commit_short" "$commit_message" "$commit_date" "$data_files"
        else
            echo "No data processing files affected in this commit."
        fi

        # Update last monitored commit
        echo $current_commit > "$LAST_COMMIT_FILE"

        # Update monitoring timestamp
        update_monitoring_timestamp "$commit_date"
    fi
}

analyze_data_impact() {
    local commit_short="$1"
    local commit_message="$2"
    local commit_date="$3"
    local changed_files="$4"

    # Determine impact level based on changed files
    local impact="Low"
    local quality_assessment="LOW IMPACT"

    if echo "$changed_files" | grep -q "models/health_metrics\|models/ios_models"; then
        impact="Critical"
        quality_assessment="CRITICAL IMPACT"
    elif echo "$changed_files" | grep -q "validation\|batch"; then
        impact="High"
        quality_assessment="HIGH IMPACT"
    elif echo "$changed_files" | grep -q "handler\|processing"; then
        impact="Medium"
        quality_assessment="MEDIUM IMPACT"
    fi

    # Generate review entry
    local review_entry="
### Commit: $commit_short - $(echo $commit_message | cut -c1-50)...
**Date:** $commit_date
**Impact:** $impact
**Data Quality Assessment:** $quality_assessment

**Files Changed:**
$(echo "$changed_files" | sed 's/^/- /')

**Validation Completeness:** ⏳ PENDING REVIEW
- Requires detailed analysis of changes
- Check for validation rule updates
- Verify data model integrity

**Data Integrity Risk:** ⏳ UNDER ASSESSMENT
- Analyzing potential data corruption risks
- Checking transaction boundary changes
- Validating error handling updates

**iOS Schema Compatibility:** ⏳ NEEDS VERIFICATION
- Verify Auto Health Export compatibility
- Check HealthKit identifier mapping
- Validate JSON schema compliance

**Batch Processing Efficiency:** ⏳ ANALYZING
- Check chunk size calculations
- Verify PostgreSQL parameter limits
- Analyze memory usage patterns

**Recommendations:**
- Detailed code review required for $(echo "$changed_files" | wc -l) changed files
- Run integration tests for affected health metrics
- Validate data processing pipeline integrity

---
"

    # Insert review entry after the commit history header
    local temp_file=$(mktemp)

    # Find line number for insertion
    local insert_line=$(grep -n "## Commit Review History" "$REVIEW_FILE" | cut -d: -f1)
    insert_line=$((insert_line + 2))

    # Insert the new review entry
    head -n $insert_line "$REVIEW_FILE" > "$temp_file"
    echo "$review_entry" >> "$temp_file"
    tail -n +$((insert_line + 1)) "$REVIEW_FILE" >> "$temp_file"

    mv "$temp_file" "$REVIEW_FILE"

    echo "Added review entry for commit $commit_short"
}

update_monitoring_timestamp() {
    local timestamp="$1"

    # Update the monitoring timestamp in the header
    sed -i "s/Last Check: [0-9-]* [0-9:]*/Last Check: $timestamp/" "$REVIEW_FILE"
    sed -i "s/\*Next check: [0-9-]* [0-9:]*\*/\*Next check: $(date -d '+30 seconds' '+%Y-%m-%d %H:%M:%S')\*/" "$REVIEW_FILE"
}

main() {
    echo "Starting Data Processing Quality Monitor..."
    echo "Monitoring interval: ${CHECK_INTERVAL} seconds"
    echo "Review file: $REVIEW_FILE"

    while true; do
        cd /mnt/datadrive_m2/self-sensored
        monitor_commits
        sleep $CHECK_INTERVAL
    done
}

# Handle script termination
trap 'echo "Data quality monitoring stopped."; exit 0' INT TERM

# Run monitoring
main