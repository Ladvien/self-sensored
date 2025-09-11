# Health Export REST API - Product Backlog

## Critical Issues (P0)

### [AUDIT-001] Rate Limiting - IP-based limit too restrictive
**Priority:** Critical  
**Points:** 3  
**AC:** 
- Increase IP-based rate limit from 20 to 100+ requests/hour
- Implement per-user rate limiting instead of per-IP
- Add retry-after header support in iOS client
**Dependencies:** None  
**Files:** src/middleware/rate_limit.rs, .env  
**Current Issue:** iOS uploads failing with 429 errors, 0/20 remaining


### [AUDIT-003] Server Availability - Cloudflare 520 errors
**Priority:** Critical  
**Points:** 2  
**AC:**
- Investigate origin server connectivity issues
- Implement health check endpoint monitoring
- Add automatic restart on failure
- Configure Cloudflare timeout settings
**Dependencies:** None  
**Files:** src/main.rs, infrastructure configs
**Current Issue:** Server returning 520 errors intermittently

## High Priority Issues (P1)

### [AUDIT-004] Database Constraint Alignment
**Priority:** High  
**Points:** 3  
**AC:**
- Update database CHECK constraints to match application validation (15-300 BPM)
- Create migration script for existing data
- Test constraint changes on production data
**Dependencies:** AUDIT-002  
**Files:** migrations/0002_health_metrics_schema.sql, migrations/0003_partitioning_setup.sql

### [AUDIT-005] Batch Processor Test Fix
**Priority:** High  
**Points:** 1  
**AC:**
- Fix test creating 8,000 activity records (exceeds 7,000 limit)
- Validate all test cases respect chunk size limits
- Add assertion to verify chunk sizes
**Dependencies:** None  
**Files:** tests/services/batch_processor_chunking_test.rs (line 246)

## Medium Priority Issues (P2)

### [AUDIT-006] iOS Data Transformation Investigation
**Priority:** Medium  
**Points:** 5  
**AC:**
- Investigate why iOS app sends heart rates 6-19 BPM
- Check for data transformation errors (division by 10?)
- Add data validation before sending from iOS
- Implement client-side retry logic
**Dependencies:** None  
**Files:** iOS app codebase

### [AUDIT-007] Enhanced Monitoring and Alerting
**Priority:** Medium  
**Points:** 3  
**AC:**
- Add metrics for validation error rates
- Alert when error rate exceeds 10%
- Monitor parameter usage per batch
- Track rate limit exhaustion events
**Dependencies:** None  
**Files:** src/middleware/metrics.rs, monitoring configs

## Low Priority Issues (P3)

### [AUDIT-008] Configuration Flexibility Enhancement
**Priority:** Low  
**Points:** 2  
**AC:**
- Add metric-specific chunk size overrides to IngestBatchConfig
- Make validation thresholds configurable via environment variables
- Document all configuration options
**Dependencies:** None  
**Files:** src/services/batch_processor.rs, src/config/

### [AUDIT-009] API Documentation Updates
**Priority:** Low  
**Points:** 1  
**AC:**
- Update OpenAPI spec with new validation ranges
- Document rate limiting behavior
- Add troubleshooting guide for common errors
**Dependencies:** AUDIT-002, AUDIT-001  
**Files:** docs/openapi.yaml, README.md

## Completed Stories

_None yet_

---

## Notes

- All stories are non-blocking and can be worked in parallel
- Critical issues should be addressed immediately to restore service
- Rate limiting issue is actively blocking iOS uploads
- Server availability issues may require infrastructure changes