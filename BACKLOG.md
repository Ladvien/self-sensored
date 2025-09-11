# Health Export REST API - Product Backlog

## Critical Issues (P0)



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



## Completed Stories

### ✅ [AUDIT-001] Rate Limiting - IP-based limit too restrictive  
**Priority:** Critical (P0)  
**Points:** 3  
**Completed:** 2025-09-11  
**Completion Details:**
- ✅ Increased IP-based rate limit from 20 to 100 requests/hour (already implemented in code, updated configuration)
- ✅ Per-user rate limiting implemented (check_user_rate_limit method with RATE_LIMIT_USE_USER_BASED flag)
- ✅ Retry-after header support implemented (middleware adds proper headers on rate limit exceeded)
- ✅ Enhanced test coverage for rate limiting scenarios
- ✅ Fixed configuration documentation to reflect actual implementation

**Implementation Notes:**
- Core implementation was already complete from previous work
- Updated .env.example to show correct IP rate limit (100 instead of 20)
- Added comprehensive tests for user vs IP rate limiting independence
- Rate limiting now properly supports both IP-based (unauthenticated) and user-based (authenticated) limits
- All acceptance criteria met and thoroughly tested

---

## Notes

- All stories are non-blocking and can be worked in parallel
- Critical issues should be addressed immediately to restore service
- Rate limiting issue is actively blocking iOS uploads
- Server availability issues may require infrastructure changes