# iOS Auto Health Export - Upload Test Guide

## Date Range: September 1-15, 2025

## 📱 Step-by-Step Upload Instructions

### 1. Configure the API Endpoint
Open **Auto Health Export** app on your iOS device and configure:

```
Settings → API Configuration

Endpoint URL: https://self-sensored-api.lolzlab.com/api/v1/ingest
Authorization: Bearer test_auto_export_key_2024
Timeout: 600 seconds
Method: POST
Content-Type: application/json
```

### 2. Test Connection First
Before uploading data:
1. Tap **Test Connection** button
2. Verify you get a success response
3. If you see "Empty payload" error - that's expected and means auth is working!

### 3. Select Date Range for Upload
1. Go to **Export** or **Upload** section
2. Select **Custom Date Range**
3. Set dates:
   - **Start Date**: September 1, 2025
   - **End Date**: September 15, 2025
4. Review estimated data size

### 4. Choose Metrics to Upload
Recommended initial test with core metrics:
- ✅ Heart Rate
- ✅ Blood Pressure (if available)
- ✅ Sleep Analysis
- ✅ Activity (Steps, Distance, Energy)
- ✅ Workouts

### 5. Start Upload
1. Tap **Upload to API** or **Export**
2. Monitor upload progress
3. Note any errors or timeout messages

## 🔍 Monitoring the Upload

### Real-Time Log Monitoring (Server Side)
While the upload is happening, I can monitor logs with:

```bash
# Watch for incoming data
tail -f /mnt/codex_fs/logs/self-sensored/app.log | grep -E "ingest|metrics|processed"

# Check for authentication
tail -f /mnt/codex_fs/logs/self-sensored/app.log | grep "authentication_success"

# Monitor for errors
tail -f /mnt/codex_fs/logs/self-sensored/app.log | grep -E "ERROR|WARN"
```

### Database Verification
After upload, check if data was stored:

```sql
-- Check raw ingestions
SELECT COUNT(*) as total_payloads,
       MIN(created_at) as first_upload,
       MAX(created_at) as last_upload
FROM raw_ingestions
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
  AND created_at >= '2025-09-17 13:00:00';

-- Check processed metrics by type
SELECT
    COUNT(*) FILTER (WHERE EXISTS (SELECT 1 FROM heart_rate_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18' AND recorded_at BETWEEN '2025-09-01' AND '2025-09-15')) as heart_rate_count,
    COUNT(*) FILTER (WHERE EXISTS (SELECT 1 FROM blood_pressure_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18' AND recorded_at BETWEEN '2025-09-01' AND '2025-09-15')) as blood_pressure_count,
    COUNT(*) FILTER (WHERE EXISTS (SELECT 1 FROM sleep_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18' AND sleep_start BETWEEN '2025-09-01' AND '2025-09-15')) as sleep_count,
    COUNT(*) FILTER (WHERE EXISTS (SELECT 1 FROM activity_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18' AND recorded_at BETWEEN '2025-09-01' AND '2025-09-15')) as activity_count,
    COUNT(*) FILTER (WHERE EXISTS (SELECT 1 FROM workout_metrics WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18' AND started_at BETWEEN '2025-09-01' AND '2025-09-15')) as workout_count
FROM users WHERE id = 'b479a3b9-9ef1-4a82-9771-adff57432e18';
```

## 📊 Expected Upload Behavior

### Success Indicators:
- HTTP 200 or 202 response from API
- Upload progress shows steady advancement
- No timeout errors after 600 seconds

### Log Entries to Expect:
```
✅ "Authentication successful: user_id=b479a3b9-9ef1-4a82-9771-adff57432e18"
✅ "Starting enhanced ingest processing"
✅ "Successfully parsed iOS format payload"
✅ "Processing N metrics for user"
✅ "Successfully processed batch"
```

### Possible Issues and Solutions:

| Issue | Solution |
|-------|----------|
| **"Invalid API key"** | Verify the Bearer token is exactly: `test_auto_export_key_2024` |
| **Connection refused** | Check internet connection and that domain resolves |
| **Timeout after 600s** | Try smaller date range (e.g., 3-5 days) |
| **"Empty payload"** | Ensure metrics are selected and date range has data |
| **SSL/Certificate error** | Verify using HTTPS not HTTP |

## 🔄 Retry Strategy

If the upload fails:
1. **Check error message** in the app
2. **Try smaller batches**: Upload 5 days at a time
3. **Check specific metrics**: Try uploading only heart rate first
4. **Verify connectivity**: Test with `curl https://self-sensored-api.lolzlab.com/health`

## 📈 Post-Upload Validation

Once upload completes, verify success:

```bash
# Quick check for new data
PGPASSWORD='37om3i*t3XfSZ0' psql -h 192.168.1.104 -U self_sensored -d self_sensored -c \
"SELECT
    'Total Raw Ingestions' as metric,
    COUNT(*) as count
FROM raw_ingestions
WHERE created_at > NOW() - INTERVAL '1 hour'
UNION ALL
SELECT
    'Heart Rate Metrics' as metric,
    COUNT(*) as count
FROM heart_rate_metrics
WHERE user_id = 'b479a3b9-9ef1-4a82-9771-adff57432e18'
    AND recorded_at BETWEEN '2025-09-01' AND '2025-09-15';"
```

## 💡 Tips for Successful Upload

1. **Start Small**: Test with 1-2 days first
2. **Stable Connection**: Use WiFi, not cellular
3. **Keep App Active**: Don't let phone sleep during upload
4. **Monitor Progress**: Watch the upload percentage
5. **Check Logs**: I'll monitor server logs during your upload

## 📞 During Upload

While you're uploading, I can:
- Monitor real-time logs for incoming data
- Check for any processing errors
- Verify data is being stored correctly
- Provide immediate feedback if issues occur

## Ready to Test?

1. ✅ API is running and healthy
2. ✅ Authentication is configured
3. ✅ Database is ready to receive data
4. ✅ Logs are being monitored

**Let me know when you start the upload and I'll monitor the logs!**

---

## Quick Reference

**API Endpoint**: `https://self-sensored-api.lolzlab.com/api/v1/ingest`
**API Key**: `test_auto_export_key_2024`
**Date Range**: September 1-15, 2025
**User**: test@lolzlab.com
**Expected Client IP**: 192.168.1.102 (from logs)