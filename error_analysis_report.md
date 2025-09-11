# Error Analysis Report - Health Export REST API
## Executive Summary

**Total Errors Analyzed**: 14 (out of 43 total ingestions)  
**Error Rate**: 32.56%  
**Time Period**: September 10-11, 2025  
**Affected User**: b0d8f483-fadf-46bb-ad54-fa694238424a  
**Total Failed Metrics**: 227,842 health data points

## Error Categories

### 1. Heart Rate Validation Errors (85.7% of errors)
- **Count**: 12 errors
- **First Seen**: September 10, 2025
- **Last Seen**: September 11, 2025 (ongoing)
- **Pattern**: Heart rate values below minimum threshold

#### Invalid Values Detected:
- **Range**: 6-19 BPM (below 20 BPM minimum)
- **Unique Invalid Values**: 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19 BPM
- **Validation Rule**: System expects 20-300 BPM

#### Affected Payloads:
| Timestamp | Metrics Count | Sample Errors |
|-----------|---------------|---------------|
| 2025-09-11 11:07:07 | 18,599 | avg_bpm: 14, 13 |
| 2025-09-11 11:05:07 | 10,908 | avg_bpm: 15, 6, 14, 16, 17 |
| 2025-09-11 11:04:17 | 11,535 | avg_bpm: 10, 18, 9, 12, 11, 13 |
| 2025-09-11 11:03:24 | 16,933 | avg_bpm: 17, 18, 15 |
| 2025-09-11 11:02:23 | 19,182 | avg_bpm: 19, 13, 12, 16, 17, 18, 14, 10, 9 |
| 2025-09-11 11:01:25 | 16,156 | avg_bpm: 17, 16, 13, 19, 18 |
| 2025-09-11 11:00:21 | 15,349 | avg_bpm: 17, 15, 19, 14, 18 |
| 2025-09-11 10:59:23 | 9,188 | avg_bpm: 16, 15, 18, 13, 17, 11, 10, 14, 7, 19 |
| 2025-09-11 10:58:33 | 14,102 | avg_bpm: 13, 18, 17, 19 |
| 2025-09-11 10:57:37 | 20,780 | avg_bpm: 18, 19 |

### 2. PostgreSQL Parameter Limit Errors (14.3% of errors)
- **Count**: 4 errors  
- **All Occurred**: September 10, 2025
- **Root Cause**: Batch size exceeding PostgreSQL's 65,535 parameter limit

#### Detailed Breakdown:
| Timestamp | Metrics Count | Parameters Used | Error |
|-----------|---------------|-----------------|-------|
| 2025-09-10 10:56:11 | 20,265 | 161,672 | Exceeded limit by 96,137 |
| 2025-09-10 10:57:01 | 16,670 | 132,800 | Exceeded limit by 67,265 |
| 2025-09-10 10:57:48 | 18,901 | 150,656 | Exceeded limit by 85,121 |
| 2025-09-10 11:29:09 | 18,901 | 150,656 | Exceeded limit by 85,121 |

**Note**: These errors indicate Activity metrics processing (7 parameters per record) with batches too large for single INSERT statements.

### 3. Duplicate Processing Error (7.1% of errors)
- **Count**: 1 error
- **Timestamp**: September 10, 2025 11:29:36
- **Error**: "ON CONFLICT DO UPDATE command cannot affect row a second time"
- **Metrics Count**: 373
- **Cause**: Attempting to update the same row multiple times in a single statement

## Payload Analysis

### Batch Size Distribution:
- **Minimum**: 373 metrics per batch
- **Maximum**: 20,780 metrics per batch
- **Average**: 15,189 metrics per batch
- **Total Failed Metrics**: 227,842

### Large Batch Issues:
- 9 out of 14 errors involved batches > 10,000 metrics
- Largest failed batch: 20,780 metrics
- These large batches are triggering both validation and parameter limit errors

## Timeline Analysis

### September 10, 2025:
- **Morning (10:56-10:57)**: PostgreSQL parameter limit errors begin
- **Late Morning (11:29)**: Duplicate processing and continued parameter errors
- **Error Types**: Primarily database constraint violations

### September 11, 2025:
- **10:57-11:07**: Continuous heart rate validation errors
- **Pattern**: Every minute, large batches (9,000-20,000 metrics) failing
- **Consistency**: All failures due to heart rates below 20 BPM

## Root Cause Analysis

### 1. Data Quality Issue
The iOS app is sending heart rate values that appear to be incorrect:
- Values of 6-19 BPM are physiologically implausible for living subjects
- Possible causes:
  - Sensor malfunction or disconnection
  - Data transformation error (possibly dividing by 10?)
  - Test data being sent from development environment

### 2. Batch Processing Configuration
The batch processor is attempting to insert too many records at once:
- Current chunking appears insufficient for Activity metrics
- Calculated need: With 7 parameters per Activity record, max safe batch is ~9,300 records
- Actual batches: Up to 20,265 Activity records attempted

### 3. Duplicate Detection
The duplicate detection mechanism is encountering edge cases where the same record appears multiple times within a single batch.

## Recommendations

### Immediate Actions:
1. **Adjust Validation Rules**: Consider if 20 BPM minimum is appropriate, or if data transformation is needed
2. **Fix Batch Chunking**: Reduce Activity batch size to 8,000 records (safe margin under parameter limit)
3. **Investigate iOS App**: Check for data collection or transformation issues causing low heart rate values

### Long-term Solutions:
1. **Implement Data Cleaning**: Add preprocessing to handle obviously erroneous values
2. **Enhanced Monitoring**: Alert on validation error rates exceeding 10%
3. **Batch Size Auto-adjustment**: Dynamically adjust batch sizes based on metric type
4. **Duplicate Handling**: Process duplicates in separate transactions

## Impact Assessment

- **Data Loss**: 227,842 health metrics not processed
- **User Impact**: Single user affected, but consistently failing for 2 days
- **System Health**: API functioning, but 90% failure rate today indicates critical issue
- **Recovery Needed**: Failed payloads preserved in raw_ingestions table for reprocessing

## Next Steps

1. Review and adjust heart rate validation thresholds
2. Deploy batch processor fixes for parameter limits
3. Investigate iOS app sensor data collection
4. Implement monitoring alerts for high error rates
5. Reprocess failed ingestions after fixes deployed