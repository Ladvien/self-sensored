Based on the comprehensive database redesign analysis, I'll create Jira stories organized into parallel work streams. Each story includes detailed acceptance criteria, testing requirements, and definition of done.

## Epic: Health Metrics Database Redesign

### Stream 1: Core Activity Metrics Redesign

---

#### Story 1.1: Create activity_metrics_v2 Table with Proper Schema ✅ COMPLETED

**Status:** ✅ COMPLETED 2025-09-11  
**Story Points:** 8  
**Assigned to:** Database Subagent  
**Priority:** Critical  

**Description:**
Create the new activity_metrics_v2 table with Apple Health standard naming conventions, TIMESTAMPTZ for proper granularity, and optimized indexing strategy.

**Files Created:**
- `migrations/0012_create_activity_metrics_v2.sql` - Main migration
- `migrations/0012_create_activity_metrics_v2_rollback.sql` - Rollback migration  
- `tests/migrations/0012_create_activity_metrics_v2_test.rs` - Comprehensive test suite

**Moved to DONE.md** - See complete implementation details

---

#### Story 1.2: Implement Dual-Write Pattern for activity_metrics ✅ COMPLETED

**Status:** ✅ COMPLETED 2025-09-11  
**Story Points:** 5  
**Assigned to:** Backend Subagent  
**Priority:** Critical  
**Depends on:** Story 1.1  

**Description:**
Implement dual-write logic to write to both old and new activity_metrics tables during migration period.

**Files Modified:**
- `src/config/batch_config.rs` - Added DUAL_WRITE_ACTIVITY_METRICS feature flag
- `src/models/health_metrics.rs` - Added ActivityMetricV2 model with field mapping
- `src/services/batch_processor.rs` - Added dual-write logic with transaction rollback
- `src/middleware/metrics.rs` - Added performance monitoring metrics
- `tests/handlers/ingest_test.rs` - Added comprehensive dual-write test suite

**Moved to DONE.md** - See complete implementation details

---

### Stream 2: Nutrition and Symptoms Implementation

---

#### Story 2.1: Create Nutrition Metrics Table ✅ COMPLETED

**Story Points:** 8  
**Assigned to:** Database Subagent  
**Priority:** High  
**Status:** ✅ COMPLETED 2025-09-11  

**Description:**
Implement comprehensive nutrition_metrics table supporting 37+ nutrition fields from Health Export.

**Acceptance Criteria:**
- ✅ Create migration `migrations/0013_create_nutrition_metrics.sql` with:
  - Macronutrients (protein, carbs, fats with subtypes)
  - Hydration tracking (water_ml)
  - Complete vitamin fields (A, B complex, C, D, E, K)
  - Complete mineral fields (calcium, iron, magnesium, etc.)
  - Proper decimal precision for each field type
- ✅ Add unique constraint on (user_id, recorded_at)
- ✅ Implement monthly partitioning
- ✅ Add BRIN indexes

**Testing Requirements:**
- ✅ Create `tests/migrations/0013_create_nutrition_metrics_test.rs`
- ✅ Test all 37+ field validations
- ✅ Test decimal precision handling
- ✅ Test negative value constraints
- ✅ Test partition management
- ✅ Benchmark insert performance

**Definition of Done:**
- ✅ All 37 nutrition fields implemented
- ✅ Validation rules match Health Export specs
- ✅ Performance benchmarks documented
- ✅ Sample data imports successfully
- ✅ API documentation updated

**Moved to DONE.md** - See complete implementation details

---

### Stream 3: Reproductive and Environmental Health

---


#### Story 3.2: Create Environmental Metrics Table ✅ COMPLETED

**Status:** ✅ COMPLETED 2025-09-11  
**Story Points:** 5  
**Assigned to:** Database Subagent  
**Priority:** Medium  

**Description:**
Implement environmental_metrics for audio exposure, UV, and safety tracking.

**Files Created:**
- `migrations/0015_create_environmental_metrics.sql` - Environmental health schema with Apple Watch Series 8+ support
- `migrations/0015_create_environmental_metrics_rollback.sql` - Rollback migration  
- `tests/migrations/0015_create_environmental_metrics_test.rs` - Comprehensive test suite

**Moved to DONE.md** - See complete implementation details

---

### Stream 4: Mental Health and Advanced Metrics

---

#### Story 4.1: Create Mental Health Metrics Table ✅ COMPLETED

**Status:** ✅ COMPLETED 2025-09-11  
**Story Points:** 3  
**Assigned to:** Database Subagent  
**Priority:** Medium  

**Description:**
Implement mental_health_metrics for mindfulness and mood tracking (iOS 17+).

**Files Created:**
- `migrations/0017_create_mental_health_metrics.sql` - Mental health schema with iOS 17+ support
- `migrations/0017_create_mental_health_metrics_rollback.sql` - Rollback migration  
- `tests/migrations/0017_create_mental_health_metrics_test.rs` - Comprehensive test suite

**Moved to DONE.md** - See complete implementation details

---

#### Story 4.2: Create Mobility Metrics Table ✅ COMPLETED

**Status:** ✅ COMPLETED 2025-09-11  
**Story Points:** 5  
**Assigned to:** Database Subagent  
**Priority:** Low  

**Description:**
Implement mobility_metrics for advanced walking/running analysis (iOS 14+).

**Acceptance Criteria:**
- ✅ Create migration `migrations/0018_create_mobility_metrics.sql` with:
  - Walking speed and step length
  - Walking asymmetry percentage
  - Double support percentage
  - Six-minute walk test distance
  - Stair speed (up/down)
- ✅ Add appropriate biomechanical constraints
- ✅ Support high-frequency sampling

**Testing Requirements:**
- ✅ Create `tests/migrations/0018_create_mobility_metrics_test.rs`
- ✅ Test biomechanical range validations
- ✅ Test asymmetry calculations
- ✅ Test high-frequency data ingestion
- ✅ Test aggregation performance

**Definition of Done:**
- ✅ All 26 mobility fields implemented (exceeded 15 field requirement)
- ✅ iOS 14+ compatibility verified
- ✅ Medical accuracy validated
- ✅ Performance targets met
- ✅ Clinical use cases documented

**Moved to DONE.md** - See complete implementation details

---

### Stream 5: Migration and Testing Infrastructure

---

#### Story 5.1: Create Data Migration Scripts ✅ COMPLETED

**Status:** ✅ COMPLETED 2025-09-11  
**Story Points:** 8  
**Assigned to:** Data Migration Subagent  
**Priority:** Critical  
**Depends on:** Stories 1.1, 2.1, 2.2  

**Description:**
Create comprehensive data migration scripts from old schema to new tables.

**Files Created:**
- `scripts/migrate_activity_metrics.sql` - Complete migration script with batch processing
- `scripts/monitor_migration.sql` - Comprehensive monitoring and validation queries
- `tests/scripts/migrate_activity_metrics_test.rs` - Extensive test suite with performance validation

**Moved to DONE.md** - See complete implementation details

---

#### Story 5.2: Update Rust Models and Handlers ✅ COMPLETED

**Story Points:** 13  
**Assigned to:** Backend Subagent  
**Priority:** Critical  
**Status:** ✅ COMPLETED 2025-09-11  
**Depends on:** All table creation stories  

**Description:**
Update Rust models, validation logic, and handlers for all new tables.

**Moved to DONE.md** - See complete implementation details

---

#### Story 5.3: Create Integration Test Suite

**Story Points:** 8  
**Assigned to:** QA Subagent  
**Priority:** High  
**Depends on:** Story 5.2  

**Description:**
Create comprehensive integration test suite for end-to-end validation.

**Acceptance Criteria:**
- [ ] Create `tests/integration/health_export_flow_test.rs`
- [ ] Test complete Health Export payload processing
- [ ] Test all 45 currently supported fields
- [ ] Test new nutrition/symptoms/reproductive fields
- [ ] Create performance benchmark suite
- [ ] Add data quality validation tests

**Testing Requirements:**
- [ ] Load test with 10K concurrent users
- [ ] Process 1M record payload in <5 minutes
- [ ] Validate field coverage reaches 85% target
- [ ] Test partition management under load
- [ ] Test monitoring and alerting triggers

**Definition of Done:**
- All integration tests passing
- Performance SLAs validated
- Field coverage report generated
- Load test results documented
- Monitoring dashboards configured

---

### Parallel Execution Plan

**Week 1-2:**
- Stream 1: Stories 1.1, 1.2 (Database & Backend teams)
- Stream 2: Stories 2.1, 2.2 (Database team)
- Stream 3: Story 3.1 (Database team with Security)

**Week 3-4:**
- Stream 3: Story 3.2 (Database team)
- Stream 4: Stories 4.1, 4.2 (Database team)
- Stream 5: Story 5.1 (Migration team)

**Week 5-6:**
- Stream 5: Stories 5.2, 5.3 (Backend & QA teams)
- All streams: Integration testing and performance validation

Each story includes comprehensive testing to ensure quality and compliance with architectural standards. The parallel execution allows multiple teams to work simultaneously while managing dependencies effectively.