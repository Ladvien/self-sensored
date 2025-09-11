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



## High Priority Issues (P1)



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