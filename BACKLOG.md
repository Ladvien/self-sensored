## **COMPLETED EMERGENCY STORIES - API Data Loss Issues RESOLVED ✅**

### **STORY-EMERGENCY-001: ✅ COMPLETED - API Status Reporting False Positives**
**Priority**: P0 - EMERGENCY (Silent data loss)
**Status**: ✅ COMPLETED 2025-09-17
**Files**: `/src/handlers/ingest.rs` lines 730-890
**Resolution**: ✅ Fixed - Payloads with data loss now correctly marked as "error"

**Root Cause**: `update_processing_status()` logic incorrectly marks payloads as successful
```rust
let status = if result.errors.is_empty() {
    "processed"  // PROBLEM: PostgreSQL rejections don't appear as "errors"
} else {
    "error"
}
```

**CRITICAL FIXES**: ✅ ALL COMPLETED
- ✅ Add actual metric count verification vs expected count from payload
- ✅ Detect PostgreSQL parameter limit rejections (not in error array)
- ✅ Mark payloads as "error" when expected != actual inserted metrics
- ✅ Add processing metadata: expected_count, actual_count, loss_percentage
- ✅ Update status to "partial_success" when some metrics fail silently

**ACCEPTANCE CRITERIA**: ✅ ALL MET
- ✅ Payloads with data loss marked as "error" or "partial_success"
- ✅ Processing metadata tracks actual vs expected metrics
- ✅ Zero false positive "processed" status for failed ingestions

**IMPLEMENTATION**: Comprehensive solution with multiple detection methods:
- Silent failure detection (expected vs actual count comparison)
- PostgreSQL parameter limit violation detection (>50 silent failures)
- Enhanced metadata tracking with detailed analysis
- Proper status determination logic with multiple thresholds
- Comprehensive logging for critical data loss scenarios

---


**ACCEPTANCE CRITERIA**:
- [ ] Empty payloads rejected with 400 status code
- [ ] No duplicate processing of identical payloads
- [ ] Clear error messages guide client developers

---

### **STORY-EMERGENCY-003: ✅ COMPLETED - Async Processing Response Misrepresentation**
**Priority**: P0 - EMERGENCY
**Status**: ✅ COMPLETED 2025-09-17
**Files**: `/src/handlers/ingest.rs` lines 148-173
**Resolution**: ✅ Fixed - Async responses now correctly indicate acceptance vs processing

**Problem**: Large payload (>10MB) async processing returned misleading success status
```rust
// PROBLEM: Response claimed false success for async processing
let response = IngestResponse {
    success: false,  // MISLEADING - request was actually accepted
    processed_count: 0,
    // ...
};
```

**CRITICAL FIXES**: ✅ ALL COMPLETED
- ✅ Changed async response success to true (request accepted)
- ✅ Kept processed_count: 0 (accurate - no processing yet)
- ✅ Added clear processing_status: "accepted_for_processing"
- ✅ Included raw_ingestion_id for status checking
- ✅ Updated response message to be clear about async nature

**ACCEPTANCE CRITERIA**: ✅ ALL MET
- ✅ Async responses don't claim false processing success
- ✅ Clear communication about processing status to clients
- ✅ Clients can check actual processing results via API

**IMPLEMENTATION**: Fixed async response fields and added comprehensive test coverage:
- Fixed success flag to indicate request acceptance (not processing completion)
- Maintained accurate processed_count: 0 for async responses
- Added comprehensive test suite in `/tests/async_processing_test.rs`
- Clear messaging directs clients to use raw_ingestion_id for status checks

---

### **STORY-EMERGENCY-004: 🚨 CRITICAL - Production Config Parameter Violations**
**Priority**: P0 - EMERGENCY (ALREADY IDENTIFIED IN STORY-CRITICAL-002)
**Status**: READY FOR IMPLEMENTATION - Root cause confirmed
**Files**: Multiple production config files

**CONFIRMED VIOLATIONS**:
- Activity: 7,000 chunk × 19 params = 133,000 > 65,535 limit (167% violation)
- Sleep: 6,000 chunk × 10 params = 60,000 > 65,535 limit (14.4% violation)
- Temperature: 8,000 chunk × 8 params = 64,000 > 65,535 limit (22.1% violation)

**EMERGENCY DEPLOYMENT REQUIRED**:
- [ ] Fix Activity chunk size: 7,000 → 2,700 in production config
- [ ] Fix Sleep chunk size: 6,000 → 5,242 in default config
- [ ] Fix Temperature chunk size: 8,000 → 6,553 in default config
- [ ] Add parameter validation to prevent future violations
- [ ] Deploy to production immediately to stop ongoing data loss

---

### **STORY-EMERGENCY-005: 📋 HIGH - Missing Environmental/AudioExposure Processing**
**Priority**: P1 - HIGH (ALREADY IDENTIFIED IN STORY-CRITICAL-001)
**Status**: READY FOR IMPLEMENTATION - Exact code locations identified
**Impact**: 100% data loss for these metric types

**EXACT IMPLEMENTATION**:
```rust
// Add to GroupedMetrics struct (batch_processor.rs ~line 3440):
environmental_metrics: Vec<crate::models::EnvironmentalMetric>,
audio_exposure_metrics: Vec<crate::models::AudioExposureMetric>,

// Add to group_metrics_by_type() function:
HealthMetric::Environmental(env) => grouped.environmental_metrics.push(env),
HealthMetric::AudioExposure(audio) => grouped.audio_exposure_metrics.push(audio),

// Add processing methods for both metric types
```

**IMMEDIATE IMPLEMENTATION READY**:
- [ ] Add missing GroupedMetrics fields
- [ ] Add missing match arms in grouping function
- [ ] Add deduplication methods
- [ ] Add batch processing methods
- [ ] Test Environmental and AudioExposure processing

---

---

## **DATA PROCESSOR FINDINGS - Comprehensive Metric Type Gap Analysis**

### **STORY-DATA-001: 🚨 CRITICAL - Complete HealthMetric Enum vs Batch Processor Gap**
**Priority**: P0 - EMERGENCY (Systematic data loss)
**Estimated Effort**: 8 hours
**Files**: `/src/services/batch_processor.rs`, `/src/models/health_metrics.rs`
**Impact**: 7+ metric types completely dropped during batch processing

**CRITICAL GAP ANALYSIS**:
The HealthMetric enum defines 15+ metric types, but batch processor only handles 11:

**✅ SUPPORTED in Batch Processor:**
- HeartRate, BloodPressure, Sleep, Activity, BodyMeasurement
- Temperature, BloodGlucose, Nutrition, Workout
- Menstrual, Fertility

**❌ MISSING in Batch Processor (100% data loss):**
- Environmental (84,432 metrics missing in REPORT.md)
- AudioExposure (1,100 metrics missing in REPORT.md)
- SafetyEvent (Fall detection, emergency SOS)
- Mindfulness (Meditation, mental wellness tracking)
- MentalHealth (Mood, anxiety, depression tracking)
- Symptom (Illness monitoring, 50+ symptom types)
- Hygiene (Personal care tracking)

**ROOT CAUSE**: `group_metrics_by_type()` function missing match arms:
```rust
// MISSING MATCH ARMS (lines ~1037-1043):
HealthMetric::Environmental(env) => grouped.environmental_metrics.push(env),
HealthMetric::AudioExposure(audio) => grouped.audio_exposure_metrics.push(audio),
HealthMetric::SafetyEvent(safety) => grouped.safety_event_metrics.push(safety),
HealthMetric::Mindfulness(mind) => grouped.mindfulness_metrics.push(mind),
HealthMetric::MentalHealth(mental) => grouped.mental_health_metrics.push(mental),
HealthMetric::Symptom(symptom) => grouped.symptom_metrics.push(symptom),
HealthMetric::Hygiene(hygiene) => grouped.hygiene_metrics.push(hygiene),
```

**IMMEDIATE TASKS**:
- [ ] **CRITICAL**: Add all missing fields to GroupedMetrics struct
- [ ] **CRITICAL**: Add all missing match arms in group_metrics_by_type()
- [ ] **CRITICAL**: Add deduplication methods for all missing metric types
- [ ] **CRITICAL**: Add batch processing methods for all missing metric types
- [ ] **CRITICAL**: Add proper chunk size calculations for each metric type
- [ ] **CRITICAL**: Add PostgreSQL parameter validation for each new type

**ACCEPTANCE CRITERIA**:
- [ ] Every HealthMetric enum variant has corresponding batch processing logic
- [ ] Zero metric types hit the `_` fallback case
- [ ] All missing metric types appear in database after processing
- [ ] Comprehensive test coverage for all metric types

---

### **STORY-DATA-002: 🔥 CRITICAL - iOS Metric Name Mapping Validation**
**Priority**: P0 - EMERGENCY (iOS integration failure)
**Estimated Effort**: 4 hours
**Files**: `/src/models/ios_models.rs`
**Impact**: Unknown iOS metric types may cause data loss

**CRITICAL FINDINGS**:
1. **Environmental vs AudioExposure Confusion**: iOS sends "environmental_audio_exposure" but creates AudioExposureMetric objects
2. **Unknown Metric Handling**: Unknown iOS metric types fall into debug logging but may not be converted
3. **Missing Metric Type Detection**: No systematic validation of iOS metric names vs internal types

**INVESTIGATION TASKS**:
- [ ] **CRITICAL**: Audit all iOS metric name mappings in `to_internal_format()` function
- [ ] **CRITICAL**: Validate every iOS metric type in REPORT.md payloads has corresponding conversion
- [ ] **CRITICAL**: Add logging for unconverted iOS metrics with sample data
- [x] **CRITICAL**: Map iOS "Environmental" type (84,432 metrics) to correct internal type ✅ COMPLETED
- [x] **CRITICAL**: Validate AudioExposure vs Environmental type routing ✅ COMPLETED

**PAYLOAD ANALYSIS TASKS**:
- [ ] Extract all unique iOS metric names from REPORT.md raw payloads
- [ ] Verify each iOS metric type has conversion logic in ios_models.rs
- [ ] Identify any iOS metric types falling into the unknown handler (`_ => {}`)
- [ ] Test iOS-to-internal conversion with sample payloads for each type

**ACCEPTANCE CRITERIA**:
- [ ] Every iOS metric type in production payloads has validated conversion
- [ ] Zero unknown/unhandled iOS metric types in production
- [ ] Clear error logging for any unsupported iOS metric types
- [ ] 100% iOS metric conversion rate verified with tests

---

### **STORY-DATA-003: ⚠️ HIGH - AudioExposure Table Architecture Fix**
**Priority**: P1 - HIGH (Database design issue)
**Estimated Effort**: 3 hours
**Files**: `/src/handlers/environmental_handler.rs`, `/database/schema.sql`
**Impact**: AudioExposure metrics incorrectly stored in environmental_metrics table

**ARCHITECTURE PROBLEM**:
Code analysis shows AudioExposure metrics being forced into environmental_metrics table:
```rust
// Line 414-425 in environmental_handler.rs:
// Note: We need to create a separate table for audio exposure or extend environmental_metrics
// For now, storing in environmental_metrics table with audio-specific fields
```

**IMMEDIATE FIXES**:
- [ ] **HIGH**: Create dedicated `audio_exposure_metrics` table in schema.sql
- [ ] **HIGH**: Update AudioExposure batch processing to use correct table
- [ ] **HIGH**: Fix environmental_handler.rs AudioExposure routing
- [ ] **HIGH**: Create migration to move existing AudioExposure data
- [ ] **HIGH**: Update handlers to separate Environmental vs AudioExposure storage

**VALIDATION TASKS**:
- [ ] Verify AudioExposure metrics stored in dedicated table
- [ ] Test Environmental and AudioExposure metrics don't cross-contaminate
- [ ] Validate table relationships and indexing for AudioExposure

**ACCEPTANCE CRITERIA**:
- [ ] AudioExposure metrics stored in proper dedicated table
- [ ] Clean separation between Environmental and AudioExposure data
- [ ] Proper indexing and performance for AudioExposure queries

---

### **STORY-DATA-004: 📋 MEDIUM - Parameter Validation vs Processing Mismatch Detection**
**Priority**: P2 - MEDIUM (System reliability)
**Estimated Effort**: 2 hours
**Files**: `/src/models/health_metrics.rs`, `/src/services/batch_processor.rs`
**Impact**: Metrics pass validation but get silently dropped

**SYSTEMATIC PROBLEM**:
`health_metrics.rs` defines comprehensive validation for all metric types, but batch processor only handles subset, creating false confidence in data processing.

**DETECTION TASKS**:
- [ ] **MEDIUM**: Create automated validation that every HealthMetric enum variant has batch processing
- [ ] **MEDIUM**: Add unit test to verify GroupedMetrics struct has field for each metric type
- [ ] **MEDIUM**: Add runtime check that no metrics hit `_` fallback in group_metrics_by_type()
- [ ] **MEDIUM**: Create monitoring alert for unsupported metric types

**PREVENTION TASKS**:
- [ ] Add compile-time check that ensures GroupedMetrics completeness
- [ ] Create integration test that validates end-to-end processing for each metric type
- [ ] Add documentation requirement: every new HealthMetric variant must include batch processing

**ACCEPTANCE CRITERIA**:
- [ ] Automated detection of validation vs processing mismatches
- [ ] Zero possibility of metrics passing validation but being dropped
- [ ] Comprehensive test coverage for all defined metric types

---

### **STORY-DATA-005: 📋 MEDIUM - iOS Metric Type Coverage Monitoring**
**Priority**: P2 - MEDIUM (Production visibility)
**Estimated Effort**: 3 hours
**Files**: Monitoring and alerting infrastructure
**Impact**: Early detection of new iOS metric types or mapping failures

**MONITORING TASKS**:
- [ ] **MEDIUM**: Add metrics tracking for iOS metric type distribution
- [ ] **MEDIUM**: Create alerts for unknown iOS metric types in production
- [ ] **MEDIUM**: Add dashboard showing iOS vs internal metric type conversion rates
- [ ] **MEDIUM**: Monitor for metrics hitting `_` fallback cases

**ALERTING TASKS**:
- [ ] Alert when new iOS metric types appear without corresponding conversion
- [ ] Alert when metric type distribution changes significantly
- [ ] Alert when conversion success rate drops below threshold

**ACCEPTANCE CRITERIA**:
- [ ] Real-time visibility into iOS metric type processing
- [ ] Early warning system for new/unsupported metric types
- [ ] Historical tracking of metric type coverage over time

---

*Last Updated: 2025-09-16*
*Total Active Stories: 20 (1 Master + 4 Emergency + 4 Critical + 5 Data Processor + 6 Others)*
*Critical Data Loss Investigation: COMPLETE - Root causes identified*
*Data Processor Analysis: COMPLETE - 7 missing metric types identified*

#### **SUB-001: CRITICAL - EnvironmentalMetric Field Alignment**
**Priority**: P0 - BLOCKING
**Lines of Code**: ~50
**Files**: `src/models/health_metrics.rs`, `src/handlers/environmental_handler.rs`
**DATA.md Ref**: Lines 178-188 (Environmental & Safety)

**Tasks**:
- [ ] Add missing audio exposure fields to EnvironmentalMetric struct
- [ ] Align with database schema `environmental_metrics` table
- [ ] Fix handler query field mapping
- [ ] Test audio exposure data ingestion

**Expected Fix**: Resolves 4+ compilation errors

#### **SUB-002: CRITICAL - DateTime Type Inference Fix**
**Priority**: P0 - BLOCKING
**Lines of Code**: ~200
**Files**: `src/handlers/temperature_handler.rs`, multiple handlers
**DATA.md Ref**: Lines 61-71 (Body Measurements - Temperature)

**Tasks**:
- [ ] Fix SQLx DateTime type annotations in all handlers
- [ ] Add explicit type casting for TIMESTAMPTZ fields
- [ ] Test timezone conversion handling
- [ ] Verify temperature metric ingestion

**Expected Fix**: Resolves 10+ compilation errors

#### **SUB-003: CRITICAL - AuthContext User ID Access**
**Priority**: P0 - BLOCKING
**Lines of Code**: ~100
**Files**: Multiple handlers accessing auth context
**DATA.md Ref**: All categories (affects all metric ingestion)

**Tasks**:
- [ ] Fix AuthContext struct to provide user_id access method
- [ ] Update all handlers using auth.user_id
- [ ] Test authentication flow
- [ ] Verify user-scoped data access

**Expected Fix**: Resolves 8+ compilation errors

#### **SUB-004: CRITICAL - Metrics Struct Field Access**
**Priority**: P0 - BLOCKING
**Lines of Code**: ~75
**Files**: Handlers using metrics for monitoring
**DATA.md Ref**: All categories (affects all metric monitoring)

**Tasks**:
- [ ] Fix Metrics struct field definitions
- [ ] Update handler metric tracking code
- [ ] Test Prometheus metrics collection
- [ ] Verify metric monitoring dashboard

**Expected Fix**: Resolves 6+ compilation errors

#### **SUB-005: HIGH - Audio Exposure Table Architecture**
**Priority**: P1 - HIGH
**Lines of Code**: ~150
**Files**: `database/schema.sql`, `src/models/health_metrics.rs`
**DATA.md Ref**: Lines 179-182 (Audio Exposure)

**Tasks**:
- [ ] Create dedicated `audio_exposure_metrics` table
- [ ] Update AudioExposureMetric struct alignment
- [ ] Implement proper table separation in handlers
- [ ] Test audio exposure storage and retrieval

**Expected Fix**: Resolves design architecture issues

#### **SUB-006: ✅ COMPLETED - Reproductive Health BatchConfig**
**Priority**: P1 - HIGH
**Lines of Code**: ~100
**Files**: `src/config/batch_config.rs`, reproductive health handlers
**DATA.md Ref**: Lines 123-137 (Reproductive Health)

**Tasks**:
- [x] Add missing reproductive health fields to BatchConfig
- [x] Update fertility and menstrual chunk sizes
- [x] Add encryption configuration fields
- [x] Test reproductive health data processing

**Expected Fix**: ✅ Resolved 3+ compilation errors

**✅ COMPLETED DELIVERABLES:**
- Fixed BatchConfig initialization in ingest_async_simple.rs with all reproductive health fields
- Optimized chunk sizes for PostgreSQL parameter limits (menstrual: 6500, fertility: 4300)
- Added comprehensive performance tests for large batch processing (12,000+ metrics)
- Fixed reproductive health handler data type mismatches
- Verified parallel processing works correctly with reproductive health metrics

#### **SUB-007: HIGH - Blood Glucose Metric Alignment**
**Priority**: P1 - HIGH
**Lines of Code**: ~75
**Files**: Blood glucose and metabolic handlers
**DATA.md Ref**: Lines 115-119 (Blood & Metabolic)

**Tasks**:
- [ ] Align BloodGlucoseMetric with database schema
- [ ] Fix metabolic handler field mappings
- [ ] Add insulin delivery tracking support
- [ ] Test blood glucose data ingestion

**Expected Fix**: Resolves 4+ compilation errors



#### **SUB-009: MEDIUM - Symptom Tracking Enhancement**
**Priority**: P2 - MEDIUM
**Lines of Code**: ~150
**Files**: Symptom handlers and models
**DATA.md Ref**: Lines 138-177 (Symptoms)

**Tasks**:
- [ ] Add all supported symptom types from DATA.md
- [ ] Implement symptom severity tracking
- [ ] Update symptom handler for comprehensive tracking
- [ ] Test symptom analysis and trends

**Expected Fix**: Improves DATA.md compliance for symptoms

#### **SUB-010: MEDIUM - Mobility Metrics Integration**
**Priority**: P2 - MEDIUM
**Lines of Code**: ~125
**Files**: Activity and mobility handlers
**DATA.md Ref**: Lines 189-202 (Mobility Metrics)

**Tasks**:
- [ ] Add walking speed, step length, asymmetry tracking
- [ ] Implement stair ascent/descent speed metrics
- [ ] Add running dynamics support
- [ ] Test mobility metric collection

**Expected Fix**: Adds new DATA.md supported metrics

#### **SUB-011: LOW - Cycling Metrics Support**
**Priority**: P3 - LOW
**Lines of Code**: ~75
**Files**: Activity handlers for cycling
**DATA.md Ref**: Lines 203-207 (Cycling Metrics)

**Tasks**:
- [ ] Add cycling speed, power, cadence tracking
- [ ] Implement functional threshold power support
- [ ] Update cycling workout analysis
- [ ] Test cycling-specific metrics

**Expected Fix**: Completes DATA.md cycling support

#### **SUB-012: LOW - Underwater Metrics Support**
**Priority**: P3 - LOW
**Lines of Code**: ~50
**Files**: Activity or specialized metrics handlers
**DATA.md Ref**: Lines 208-209 (Underwater)

**Tasks**:
- [ ] Add underwater depth tracking support
- [ ] Implement diving metric collection
- [ ] Test underwater activity tracking
- [ ] Verify iOS 16+ compatibility

**Expected Fix**: Adds niche but supported DATA.md metric

### **Validation Criteria:**

1. **Compilation Success**: All 56 compilation errors resolved
2. **DATA.md Compliance**: All ✅ supported health data types properly modeled
3. **Database Alignment**: All struct fields map to database schema fields
4. **Handler Functionality**: All API endpoints compile and basic tests pass
5. **Type Safety**: SQLx macros compile without type inference errors

### **Dependencies:**
- None (blocking all other development)

### **Acceptance Criteria:**
- [ ] `cargo check` passes without errors
- [ ] All DATA.md ✅ supported health types have corresponding struct definitions
- [ ] Database schema matches struct field definitions
- [ ] All handlers compile and can handle basic requests
- [ ] Basic integration tests pass for core metric ingestion
- [ ] SQLx query preparation succeeds
- [ ] Authentication and metrics monitoring work

### **Definition of Done:**
- [ ] Zero compilation errors
- [ ] All core health metrics align with DATA.md specification
- [ ] Database schema supports all defined struct fields
- [ ] API handlers can ingest data for all supported metrics
- [ ] Integration tests pass for critical data paths
- [ ] Code review approved by Data Architecture team
- [ ] Documentation updated for data model alignment

---

## CRITICAL DATA LOSS ISSUES - 52.9% Data Loss Investigation

### **STORY-CRITICAL-001: 🚨 EMERGENCY - Batch Processor Missing Metric Types**
**Priority**: P0 - EMERGENCY (1.4M metrics lost)
**Estimated Effort**: 4 hours
**Files**: `/src/services/batch_processor.rs`
**Data Loss**: 100% Environmental (84,432) + AudioExposure (1,100) metrics

**Root Cause**: Batch processor missing fields in GroupedMetrics struct and match arms
**Evidence**: REPORT.md shows metrics parsed from iOS but never inserted to DB

**IMMEDIATE TASKS**:
- [ ] Add `environmental_metrics: Vec<EnvironmentalMetric>` to GroupedMetrics (Line 3440)
- [ ] Add `audio_exposure_metrics: Vec<AudioExposureMetric>` to GroupedMetrics (Line 3441)
- [ ] Add Environmental match arm to `group_metrics_by_type()` function
- [ ] Add AudioExposure match arm to `group_metrics_by_type()` function
- [ ] Add deduplication methods for Environmental and AudioExposure metrics
- [ ] Add batch processing methods for both metric types

**ACCEPTANCE CRITERIA**:
- [ ] Environmental metrics appear in database after processing
- [ ] AudioExposure metrics stored in correct table (not environmental_metrics)
- [ ] Zero data loss for these metric types in test payloads
- [ ] Batch processor logs show all metric types being processed

---

### **STORY-CRITICAL-002: 🔥 EMERGENCY - Activity Metrics PostgreSQL Parameter Limit Exceeded**
**Priority**: P0 - EMERGENCY (1.3M metrics lost)
**Estimated Effort**: 2 hours (Root cause identified)
**Files**: `/src/handlers/ingest_async_simple.rs`, test files with unsafe chunk sizes
**Data Loss**: 1,327,987 Activity metrics missing (51% loss rate)

**✅ ROOT CAUSE IDENTIFIED**: PostgreSQL parameter limit exceeded
- **Production Config**: `activity_chunk_size: 7000` in ingest_async_simple.rs
- **Parameter Calculation**: 7,000 records × 19 params = **133,000 parameters**
- **PostgreSQL Limit**: 65,535 parameters maximum
- **Violation**: 133,000 exceeds limit by **167%**, causing silent batch failures

**IMMEDIATE FIXES**:
- [ ] **CRITICAL**: Change `activity_chunk_size: 7000` to `2700` in `/src/handlers/ingest_async_simple.rs` line 189
- [ ] Fix unsafe test configurations using 6500, 7000 chunk sizes
- [ ] Add PostgreSQL parameter validation in BatchConfig.validate()
- [ ] Add explicit error detection for oversized PostgreSQL queries
- [ ] Update chunk size comments to reflect correct parameter calculations

**VALIDATION TASKS**:
- [ ] Test that 2700 chunk size processes correctly (51,300 params < 65,535 limit)
- [ ] Verify all test configurations use safe chunk sizes
- [ ] Add integration test for parameter limit edge cases
- [ ] Monitor Activity metrics processing success rate

**ACCEPTANCE CRITERIA**:
- [ ] Activity metrics data loss reduced to <5%
- [ ] No PostgreSQL parameter limit violations in production
- [ ] All test configurations use safe chunk sizes
- [ ] Successful processing of large Activity metric batches in tests

---

### **STORY-CRITICAL-003: ⚠️ HIGH - Audio Exposure Table Missing**
**Priority**: P1 - HIGH
**Estimated Effort**: 3 hours
**Files**: `/database/schema.sql`, `/src/handlers/environmental_handler.rs`
**Data Loss**: AudioExposure metrics incorrectly stored in environmental_metrics

**Root Cause**: No dedicated audio_exposure_metrics table, causing data architecture confusion
**Evidence**: Code shows AudioExposure stored in environmental_metrics table

**IMMEDIATE TASKS**:
- [ ] Create `audio_exposure_metrics` table in schema.sql
- [ ] Update environmental_handler.rs to use correct table for AudioExposure
- [ ] Add indexes for audio_exposure_metrics table
- [ ] Create migration script to move existing AudioExposure data
- [ ] Update batch processor to target correct table

**ACCEPTANCE CRITERIA**:
- [ ] AudioExposure metrics stored in dedicated table
- [ ] Environmental and AudioExposure data properly separated
- [ ] No cross-contamination between metric types

---

### **STORY-CRITICAL-004: ⚠️ HIGH - HeartRate Metrics 41% Data Loss**
**Priority**: P1 - HIGH
**Estimated Effort**: 4 hours
**Files**: `/src/services/batch_processor.rs`
**Data Loss**: 659 HeartRate metrics missing (41% loss rate)

**Root Cause**: Similar to Activity metrics - likely batch processing or validation issues
**Evidence**: REPORT.md shows 1,603 expected vs 944 in database

**INVESTIGATION TASKS**:
- [ ] Check HeartRate metrics chunk size and batch processing
- [ ] Add error logging for HeartRate batch insert failures
- [ ] Review HeartRate validation constraints
- [ ] Test HeartRate deduplication logic for edge cases
- [ ] Check for constraint violations on advanced cardiovascular fields

**ACCEPTANCE CRITERIA**:
- [ ] HeartRate metrics data loss reduced to <5%
- [ ] Clear error logging for any failed HeartRate inserts

---

### **STORY-CRITICAL-005: 📋 MEDIUM - Data Recovery and Reprocessing**
**Priority**: P2 - MEDIUM (after fixing causes)
**Estimated Effort**: 8 hours
**Files**: Raw payload reprocessing utility
**Scope**: Recover 1.4M missing metrics from raw_ingestions table

**RECOVERY TASKS**:
- [ ] Create reprocessing utility for raw_ingestions table
- [ ] Reprocess all payloads with fixed batch processor
- [ ] Verify recovered metrics match REPORT.md expected counts
- [ ] Add payload-to-database verification checksums
- [ ] Implement monitoring alerts for processing discrepancies

**ACCEPTANCE CRITERIA**:
- [ ] All missing metrics recovered from raw payloads
- [ ] Future processing monitored to prevent data loss
- [ ] Verification checksums in place for data integrity

---

### **STORY-OPTIMIZATION-001: 🚀 Batch Processing Parameter Optimization**
**Priority**: P2 - MEDIUM (Performance optimization)
**Estimated Effort**: 4 hours
**Files**: `/src/config/batch_config.rs`, test configurations
**Goal**: Optimize all metric chunk sizes for maximum performance while staying safe

**OPTIMIZATION ANALYSIS**:
Current configurations vs optimal safe limits (80% of PostgreSQL max 65,535):

| Metric Type | Current Chunk | Params/Record | Current Max Params | Safe Limit | Status & Opportunity |
|-------------|---------------|---------------|-------------------|------------|---------------------|
| HeartRate | 4,200 | 10 | 42,000 | 52,428 | ✅ Can increase to 5,242 (+25%) |
| BloodPressure | 8,000 | 6 | 48,000 | 52,428 | ✅ Can increase to 8,738 (+9%) |
| Sleep | 6,000 | 10 | 60,000 | 52,428 | ❌ **UNSAFE** - reduce to 5,242 |
| Activity | 2,700 | 19 | 51,300 | 52,428 | ✅ Near optimal (+2% possible) |
| BodyMeasurement | 3,000 | 16 | 48,000 | 52,428 | ✅ Can increase to 3,276 (+9%) |
| Temperature | 8,000 | 8 | 64,000 | 52,428 | ❌ **UNSAFE** - reduce to 6,553 |
| Respiratory | 7,000 | 7 | 49,000 | 52,428 | ✅ Can increase to 7,489 (+7%) |
| Nutrition | 1,600 | 32 | 51,200 | 52,428 | ✅ Can increase to 1,638 (+2%) |

**CRITICAL FIXES (UNSAFE CONFIGURATIONS)**:
- [ ] **CRITICAL**: Fix Sleep chunk size from 6,000 to 5,242 (exceeds limit by 14.4%)
- [ ] **CRITICAL**: Fix Temperature chunk size from 8,000 to 6,553 (exceeds limit by 22.1%)

**PERFORMANCE OPTIMIZATIONS**:
- [ ] Optimize HeartRate chunk size from 4,200 to 5,242 (+25% throughput)
- [ ] Optimize BloodPressure chunk size from 8,000 to 8,738 (+9% throughput)
- [ ] Optimize BodyMeasurement chunk size from 3,000 to 3,276 (+9% throughput)
- [ ] Optimize Respiratory chunk size from 7,000 to 7,489 (+7% throughput)
- [ ] Fix all test configurations to use safe, optimized chunk sizes
- [ ] Add runtime parameter validation with detailed error messages
- [ ] Create performance benchmarks for optimized configurations

**SAFETY VALIDATION**:
- [ ] Add unit tests for parameter limit calculations
- [ ] Test edge cases near PostgreSQL parameter limit
- [ ] Verify optimized configurations in load testing
- [ ] Add monitoring for parameter usage metrics

**ACCEPTANCE CRITERIA**:
- [ ] All chunk sizes optimized for maximum safe performance
- [ ] Zero risk of PostgreSQL parameter limit violations
- [ ] Measurable performance improvement in large batch processing
- [ ] All configurations validated with automated tests

---

## Additional Stories (Awaiting Master Story Completion)

*All other development blocked until STORY-MASTER-001 compilation issues resolved.*

## Legend
- 🔥 CRITICAL: Blocking compilation/deployment
- ⚠️ HIGH: Major functionality affected
- 📋 MEDIUM: Feature enhancement or optimization
- 💡 LOW: Nice-to-have improvements

---

