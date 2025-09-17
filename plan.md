# Health Export API Recovery Plan

## Current Issues Summary

### 🔴 Critical Issues
1. **Application Not Running** - Server is completely down, no listener on port 9876
2. **No API Keys** - Authentication system has no valid API keys configured
3. **Empty Database** - All tables truncated, including raw_ingestions (35 records lost)
4. **iOS App Failures** - Continuous timeout errors for Sept 1-17 data uploads
5. **Application Crashes** - Logs show repeated startup attempts but immediate failures

### 📊 Data Loss Impact
- **35 raw ingestion payloads** - Lost due to CASCADE truncation
- **1,257,952 activity metrics** - Truncated
- **944 heart rate metrics** - Truncated
- **9 body measurements** - Truncated
- **2 users preserved** - Only remaining data

## Recovery Plan

### Phase 1: Application Stabilization (Immediate)

#### 1.1 Fix Application Startup Issues
```bash
# Check for compilation errors
cargo check

# Fix any compilation warnings (unused variables)
cargo fix --allow-dirty

# Build release version
cargo build --release

# Test startup locally
./target/release/self-sensored
```

#### 1.2 Diagnose Crash Cause
- Check for missing environment variables
- Verify database connectivity
- Check Redis connectivity (currently failing)
- Review startup logs for panic messages

#### 1.3 Create Systemd Service (Optional but Recommended)
```bash
# Create service file at /etc/systemd/system/self-sensored.service
[Unit]
Description=Health Export REST API
After=network.target postgresql.service

[Service]
Type=simple
User=ladvien
WorkingDirectory=/mnt/datadrive_m2/self-sensored
Environment="PATH=/home/ladvien/.cargo/bin:/usr/local/bin:/usr/bin:/bin"
ExecStart=/mnt/datadrive_m2/self-sensored/target/release/self-sensored
Restart=on-failure
RestartSec=10
StandardOutput=append:/mnt/codex_fs/logs/self-sensored/service.log
StandardError=append:/mnt/codex_fs/logs/self-sensored/service-error.log

[Install]
WantedBy=multi-user.target
```

### Phase 2: Authentication Setup

#### 2.1 Generate API Keys for Users
```bash
# For user: test@lolzlab.com (ID: b479a3b9-9ef1-4a82-9771-adff57432e18)
cargo run --bin generate_api_key -- \
  --user-id "b479a3b9-9ef1-4a82-9771-adff57432e18" \
  --name "iOS Auto Export App"

# Store the generated key securely - this is what goes in the iOS app
```

#### 2.2 Configure iOS App
1. Open Auto Export app settings
2. Set API endpoint: `http://192.168.1.104:9876/api/v1/ingest`
3. Add API key header: `X-API-Key: <generated_key>`
4. Set timeout to 600 seconds (matching server config)

### Phase 3: Data Recovery

#### 3.1 Import Test Data (110,844 metrics available)
```python
#!/usr/bin/env python3
# File: import_test_data.py

import json
import requests
import time
from pathlib import Path

# Configuration
API_URL = "http://192.168.1.104:9876/api/v1/ingest-async"
API_KEY = "YOUR_GENERATED_API_KEY_HERE"  # Replace with actual key
TEST_DATA_FILE = "/mnt/datadrive_m2/self-sensored/test_data/auto_health_export_sample.json"
CHUNK_SIZE = 1000  # Process 1000 metrics at a time

def import_data():
    # Load test data
    with open(TEST_DATA_FILE, 'r') as f:
        data = json.load(f)

    metrics = data['data']['metrics']
    total = len(metrics)
    print(f"Found {total} metrics to import")

    # Process in chunks
    for i in range(0, total, CHUNK_SIZE):
        chunk = metrics[i:i+CHUNK_SIZE]
        payload = {
            "data": {
                "metrics": chunk,
                "workouts": []
            }
        }

        headers = {
            "X-API-Key": API_KEY,
            "Content-Type": "application/json"
        }

        try:
            response = requests.post(
                API_URL,
                json=payload,
                headers=headers,
                timeout=120
            )

            if response.status_code == 200:
                print(f"✓ Imported {i+len(chunk)}/{total} metrics")
            else:
                print(f"✗ Failed at {i}: {response.status_code} - {response.text}")
                break

        except Exception as e:
            print(f"✗ Error at {i}: {e}")
            break

        time.sleep(0.5)  # Rate limiting

if __name__ == "__main__":
    import_data()
```

#### 3.2 Verify Data Processing
```sql
-- Check raw_ingestions
SELECT COUNT(*) as ingestions,
       COUNT(DISTINCT payload_hash) as unique_payloads,
       MIN(created_at) as first_ingestion,
       MAX(created_at) as last_ingestion
FROM raw_ingestions;

-- Check metric tables
SELECT
    'activity_metrics' as table_name, COUNT(*) as count FROM activity_metrics
UNION ALL
    SELECT 'heart_rate_metrics', COUNT(*) FROM heart_rate_metrics
UNION ALL
    SELECT 'environmental_metrics', COUNT(*) FROM environmental_metrics;
```

### Phase 4: Production Deployment

#### 4.1 Enable Monitoring
```bash
# Check metrics endpoint
curl http://localhost:9090/metrics

# Monitor logs
tail -f /mnt/codex_fs/logs/self-sensored/self-sensored.log
```

#### 4.2 Setup Health Checks
```bash
# Create monitoring script
#!/bin/bash
# File: /opt/scripts/health_check.sh

# Check if service is running
if ! curl -f http://localhost:9876/health > /dev/null 2>&1; then
    echo "Service down, restarting..."
    systemctl restart self-sensored
fi
```

#### 4.3 Configure Automated Backups
```bash
# Daily backup of raw_ingestions
0 2 * * * pg_dump -h 192.168.1.104 -U self_sensored -t raw_ingestions self_sensored | gzip > /backup/raw_ingestions_$(date +\%Y\%m\%d).sql.gz
```

### Phase 5: iOS App Re-upload

#### 5.1 Trigger Historical Data Upload
1. Open Auto Export app
2. Go to Settings → Upload History
3. Select date range: Sept 1-17, 2025
4. Tap "Re-upload Selected Days"
5. Monitor server logs for processing

#### 5.2 Monitor Upload Progress
```bash
# Watch real-time logs
tail -f /mnt/codex_fs/logs/self-sensored/self-sensored.log | \
  grep -E "ingest|processed|metrics"

# Check database progress
watch -n 5 'psql -c "SELECT COUNT(*) FROM raw_ingestions"'
```

## Success Criteria

### ✅ Checklist
- [ ] Application runs without crashing
- [ ] Health endpoint responds: `curl http://localhost:9876/health`
- [ ] API key generated and stored securely
- [ ] iOS app configured with correct endpoint and API key
- [ ] Test data successfully imported (110,844 metrics)
- [ ] Raw ingestions table has records
- [ ] Metric tables are populating correctly
- [ ] iOS app can upload without timeout errors
- [ ] Historical data (Sept 1-17) uploaded successfully

## Rollback Plan

If issues persist:

1. **Stop the service**
   ```bash
   systemctl stop self-sensored
   ```

2. **Restore from the test data**
   - Use the Python import script with test data

3. **Check for data integrity**
   ```sql
   -- Verify no data corruption
   SELECT COUNT(*), COUNT(DISTINCT user_id), COUNT(DISTINCT payload_hash)
   FROM raw_ingestions;
   ```

4. **Contact support if needed**
   - Review logs for panic messages
   - Check GitHub issues for similar problems
   - Consider reverting recent commits if necessary

## Timeline

| Phase | Task | Duration | Priority |
|-------|------|----------|----------|
| 1 | Fix application startup | 30 min | CRITICAL |
| 2 | Generate API keys | 10 min | CRITICAL |
| 3 | Import test data | 20 min | HIGH |
| 4 | Configure monitoring | 15 min | MEDIUM |
| 5 | iOS app re-upload | 1-2 hours | HIGH |

**Total estimated time**: 2-3 hours

## Notes

- The application has been unstable since Sept 15
- Redis connection is failing but falling back to in-memory rate limiting
- Consider implementing a health check endpoint monitor
- Document the API key in a secure location
- Consider implementing automated database backups before any future TRUNCATE operations