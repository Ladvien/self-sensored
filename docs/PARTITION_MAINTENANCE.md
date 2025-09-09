# Partition Maintenance Documentation

## Overview

The Health Export API uses PostgreSQL monthly partitioning for time-series data to ensure optimal performance and management of large datasets. This document provides comprehensive procedures for maintaining partitioned tables.

## Partitioned Tables

The following tables use monthly partitioning:

- `raw_ingestions_partitioned` - Partitioned by `received_at`
- `audit_log_partitioned` - Partitioned by `created_at`  
- `heart_rate_metrics_partitioned` - Partitioned by `recorded_at`
- `blood_pressure_metrics_partitioned` - Partitioned by `recorded_at`
- `sleep_metrics_partitioned` - Partitioned by `date`
- `activity_metrics_partitioned` - Partitioned by `recorded_at`

## Automated Partition Management

### Partition Creation Function

The `maintain_partitions()` function automatically creates partitions for the next 12 months:

```sql
-- Run this monthly via cron job
SELECT maintain_partitions();
```

### Setting Up Cron Job

Add this to your PostgreSQL server's crontab:

```bash
# Create future partitions on the 1st of each month at 2 AM
0 2 1 * * psql -d health_export -c "SELECT maintain_partitions();"
```

## Manual Partition Operations

### Creating Partitions Manually

To create partitions for a specific table:

```sql
-- Create partitions for the next 6 months
SELECT create_monthly_partitions('raw_ingestions_partitioned', 'received_at', 0, 6);

-- Create partitions going back 2 months and forward 12 months
SELECT create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at', 2, 12);
```

### Creating a Single Partition

```sql
-- Example: Create partition for January 2025
CREATE TABLE raw_ingestions_partitioned_2025_01 PARTITION OF raw_ingestions_partitioned 
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
```

### Listing Existing Partitions

```sql
-- View all partitions for a specific table
SELECT 
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) as size
FROM pg_tables 
WHERE tablename LIKE 'raw_ingestions_partitioned_%'
ORDER BY tablename;

-- View partition constraints
SELECT 
    t.relname as partition_name,
    pg_get_expr(c.relpartbound, c.oid) as partition_constraint
FROM pg_class t
JOIN pg_inherits i ON i.inhrelid = t.oid
JOIN pg_class p ON i.inhparent = p.oid
JOIN pg_class c ON c.oid = t.oid
WHERE p.relname = 'raw_ingestions_partitioned'
ORDER BY t.relname;
```

## Data Retention and Cleanup

### Automated Cleanup

Use the `drop_old_partitions()` function to remove old data:

```sql
-- Drop partitions older than 24 months (default)
SELECT drop_old_partitions('raw_ingestions_partitioned');

-- Drop partitions older than 12 months
SELECT drop_old_partitions('heart_rate_metrics_partitioned', 12);
```

### Setting Up Automated Cleanup

Add to cron to run quarterly:

```bash
# Drop old partitions every 3 months on the 1st at 3 AM
0 3 1 */3 * psql -d health_export -c "SELECT drop_old_partitions('raw_ingestions_partitioned', 24);"
```

### Manual Partition Removal

```sql
-- Remove a specific partition
DROP TABLE IF EXISTS raw_ingestions_partitioned_2023_01;

-- Remove multiple old partitions
DO $$
DECLARE
    partition_record record;
BEGIN
    FOR partition_record IN 
        SELECT tablename 
        FROM pg_tables 
        WHERE tablename LIKE 'raw_ingestions_partitioned_2023_%'
    LOOP
        EXECUTE format('DROP TABLE IF EXISTS %I', partition_record.tablename);
        RAISE NOTICE 'Dropped partition: %', partition_record.tablename;
    END LOOP;
END
$$;
```

## Index Maintenance

### BRIN Index Maintenance

BRIN indexes need periodic refresh to maintain optimal performance:

```sql
-- Refresh all BRIN indexes
SELECT refresh_brin_indexes();

-- Refresh specific BRIN index
REINDEX INDEX idx_heart_rate_partitioned_time_brin;
```

### Creating Indexes on New Partitions

When creating partitions manually, add appropriate indexes:

```sql
-- Example for a new raw_ingestions partition
SELECT create_partition_indexes('raw_ingestions_partitioned_2025_06', 'raw_ingestions_partitioned');
```

### Setting Up BRIN Index Refresh

Add to cron to refresh weekly:

```bash
# Refresh BRIN indexes every Sunday at 4 AM
0 4 * * 0 psql -d health_export -c "SELECT refresh_brin_indexes();"
```

## Monitoring and Maintenance

### Partition Size Monitoring

```sql
-- Check partition sizes
SELECT 
    tablename,
    pg_size_pretty(pg_total_relation_size('public.'||tablename)) as size,
    pg_total_relation_size('public.'||tablename) as size_bytes
FROM pg_tables 
WHERE tablename LIKE '%_partitioned_%'
ORDER BY pg_total_relation_size('public.'||tablename) DESC;
```

### Data Distribution Analysis

```sql
-- Check data distribution across partitions
SELECT 
    tablename,
    (xpath('/row/c/text()', query_to_xml('SELECT count(*) as c FROM public.' || tablename, false, true, '')))[1]::text::int as record_count
FROM pg_tables 
WHERE tablename LIKE 'heart_rate_metrics_partitioned_%'
ORDER BY tablename;
```

### Query Performance Monitoring

```sql
-- Check if queries are using partition pruning
EXPLAIN (ANALYZE, BUFFERS)
SELECT COUNT(*) 
FROM raw_ingestions_partitioned 
WHERE received_at >= '2025-01-01' 
AND received_at < '2025-02-01';
```

## Troubleshooting

### Partition Pruning Not Working

If queries are scanning all partitions instead of just relevant ones:

1. Ensure WHERE clauses use the partition key directly
2. Check that constraint exclusion is enabled:
   ```sql
   SHOW constraint_exclusion; -- Should be 'partition' or 'on'
   ```
3. Update table statistics:
   ```sql
   ANALYZE raw_ingestions_partitioned;
   ```

### Missing Partitions

If data insertion fails due to missing partitions:

```sql
-- Create emergency partition for current month
SELECT create_monthly_partitions('raw_ingestions_partitioned', 'received_at', 0, 1);
```

### Partition Constraint Violations

To find and fix constraint violations:

```sql
-- Find records that don't match partition constraints
SELECT 
    tableoid::regclass as partition_name,
    COUNT(*) as violating_records
FROM raw_ingestions_partitioned
WHERE received_at < '2025-01-01' OR received_at >= '2025-02-01'
GROUP BY tableoid;
```

## Performance Optimization

### Optimizing Partition Pruning

- Always include the partition key in WHERE clauses
- Use exact date ranges when possible
- Avoid functions on partition keys in WHERE clauses

```sql
-- Good: Uses partition pruning
SELECT * FROM heart_rate_metrics_partitioned 
WHERE recorded_at >= '2025-01-01' AND recorded_at < '2025-02-01';

-- Bad: May not use partition pruning
SELECT * FROM heart_rate_metrics_partitioned 
WHERE DATE(recorded_at) = '2025-01-15';
```

### Bulk Data Operations

For large data imports or deletions, work with individual partitions:

```sql
-- Efficient bulk insert into specific partition
INSERT INTO heart_rate_metrics_partitioned_2025_01 
SELECT * FROM staging_heart_rate 
WHERE recorded_at >= '2025-01-01' AND recorded_at < '2025-02-01';
```

## Backup and Recovery

### Partition-Level Backups

```bash
# Backup specific partition
pg_dump -t raw_ingestions_partitioned_2025_01 health_export > partition_2025_01.sql

# Restore specific partition
psql health_export < partition_2025_01.sql
```

### Full Table Backup

```bash
# Backup entire partitioned table (includes all partitions)
pg_dump -t raw_ingestions_partitioned health_export > full_partitioned_table.sql
```

## Migration Procedures

### Adding New Partitioned Column

1. Add column to parent table:
   ```sql
   ALTER TABLE raw_ingestions_partitioned ADD COLUMN new_field TEXT;
   ```

2. Column automatically appears in all existing partitions

3. Update partition creation function if needed for indexes

### Converting Non-Partitioned to Partitioned

See migration file `0006_data_migration.sql` for the complete procedure.

## Monitoring Queries

### Daily Health Check

```sql
-- Run this daily to check partition health
SELECT 
    'Partition Health Check' as check_type,
    COUNT(*) as total_partitions,
    MIN(tablename) as oldest_partition,
    MAX(tablename) as newest_partition
FROM pg_tables 
WHERE tablename LIKE '%_partitioned_%';

-- Check for missing expected partitions  
WITH expected_partitions AS (
    SELECT 
        'raw_ingestions_partitioned_' || to_char(generate_series(
            date_trunc('month', CURRENT_DATE - INTERVAL '1 month'),
            date_trunc('month', CURRENT_DATE + INTERVAL '12 months'),
            '1 month'::interval
        ), 'YYYY_MM') as expected_name
)
SELECT ep.expected_name
FROM expected_partitions ep
LEFT JOIN pg_tables pt ON pt.tablename = ep.expected_name
WHERE pt.tablename IS NULL;
```

### Weekly Performance Check

```sql
-- Weekly query performance analysis
SELECT 
    tablename,
    pg_stat_get_tuples_inserted(oid) as inserts_this_week,
    pg_stat_get_tuples_updated(oid) as updates_this_week,
    pg_stat_get_tuples_deleted(oid) as deletes_this_week
FROM pg_class c
JOIN pg_namespace n ON c.relnamespace = n.oid
WHERE n.nspname = 'public'
AND c.relname LIKE '%_partitioned_%'
ORDER BY pg_stat_get_tuples_inserted(oid) DESC;
```

## Emergency Procedures

### Emergency Partition Creation

If the application is failing due to missing partitions:

```sql
-- Create partitions for current and next month immediately
SELECT create_monthly_partitions('raw_ingestions_partitioned', 'received_at', 0, 2);
SELECT create_monthly_partitions('heart_rate_metrics_partitioned', 'recorded_at', 0, 2);
SELECT create_monthly_partitions('audit_log_partitioned', 'created_at', 0, 2);
```

### Fixing Corrupted Partitions

```sql
-- If a partition is corrupted, detach and recreate
ALTER TABLE raw_ingestions_partitioned DETACH PARTITION raw_ingestions_partitioned_2025_01;
DROP TABLE raw_ingestions_partitioned_2025_01;
CREATE TABLE raw_ingestions_partitioned_2025_01 PARTITION OF raw_ingestions_partitioned 
    FOR VALUES FROM ('2025-01-01') TO ('2025-02-01');
```

---

**Note**: Always test partition maintenance operations in a development environment before running in production. Consider application downtime requirements when performing major partition operations.