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

#### Story 2.1: Create Nutrition Metrics Table

**Story Points:** 8  
**Assigned to:** Database Subagent  
**Priority:** High  

**Description:**
Implement comprehensive nutrition_metrics table supporting 35+ nutrition fields from Health Export.

**Acceptance Criteria:**
- [ ] Create migration `migrations/0012_create_nutrition_metrics.sql` with:
  - Macronutrients (protein, carbs, fats with subtypes)
  - Hydration tracking (water_ml)
  - Complete vitamin fields (A, B complex, C, D, E, K)
  - Complete mineral fields (calcium, iron, magnesium, etc.)
  - Proper decimal precision for each field type
- [ ] Add unique constraint on (user_id, recorded_at)
- [ ] Implement monthly partitioning
- [ ] Add BRIN indexes

**Testing Requirements:**
- [ ] Create `tests/migrations/0012_create_nutrition_metrics_test.rs`
- [ ] Test all 35+ field validations
- [ ] Test decimal precision handling
- [ ] Test negative value constraints
- [ ] Test partition management
- [ ] Benchmark insert performance

**Definition of Done:**
- All 35 nutrition fields implemented
- Validation rules match Health Export specs
- Performance benchmarks documented
- Sample data imports successfully
- API documentation updated

---

#### Story 2.2: Create Symptoms Tracking Table

**Story Points:** 5  
**Assigned to:** Database Subagent  
**Priority:** High  

**Description:**
Implement symptoms table for 35+ symptom types with severity tracking.

**Acceptance Criteria:**
- [ ] Create migration `migrations/0013_create_symptoms.sql` with:
  - Symptom_type field with enumeration
  - Severity scale (mild, moderate, severe, not_present)
  - Duration tracking
  - Notes field for additional context
- [ ] Add composite indexes for (user_id, symptom_type, recorded_at)
- [ ] Implement monthly partitioning
- [ ] Add symptom type validation

**Testing Requirements:**
- [ ] Create `tests/migrations/0013_create_symptoms_test.rs`
- [ ] Test all 35 symptom type enumerations
- [ ] Test severity validation
- [ ] Test query performance for symptom history
- [ ] Test concurrent symptom logging

**Definition of Done:**
- All symptom types enumerated
- Severity validation enforced
- Query performance <50ms for 3-month history
- Sample symptom data imports
- Clinical compliance reviewed

---

### Stream 3: Reproductive and Environmental Health

---

#### Story 3.1: Create Reproductive Health Table

**Story Points:** 8  
**Assigned to:** Database Subagent  
**Priority:** High  

**Description:**
Implement reproductive_health table with privacy-sensitive field handling.

**Acceptance Criteria:**
- [ ] Create migration `migrations/0014_create_reproductive_health.sql` with:
  - Menstrual tracking fields (flow, spotting, cycle_day)
  - Fertility tracking (basal_body_temp, cervical_mucus, ovulation)
  - Pregnancy tracking fields
  - Sexual health fields (marked for encryption)
- [ ] Implement field-level encryption for sensitive fields
- [ ] Add audit logging triggers
- [ ] Add privacy access controls

**Testing Requirements:**
- [ ] Create `tests/migrations/0014_create_reproductive_health_test.rs`
- [ ] Test encryption/decryption of sensitive fields
- [ ] Test audit log generation
- [ ] Test access control restrictions
- [ ] Test data retention policies

**Definition of Done:**
- All fields encrypted as specified
- Audit logging verified
- HIPAA compliance checklist completed
- Privacy impact assessment documented
- User consent flow implemented

---

#### Story 3.2: Create Environmental Metrics Table

**Story Points:** 5  
**Assigned to:** Database Subagent  
**Priority:** Medium  

**Description:**
Implement environmental_metrics for audio exposure, UV, and safety tracking.

**Acceptance Criteria:**
- [ ] Create migration `migrations/0015_create_environmental_metrics.sql` with:
  - Audio exposure fields (environmental, headphone, reduction)
  - UV exposure tracking
  - Fall detection events
  - Hygiene tracking (handwashing, toothbrushing)
- [ ] Add appropriate value constraints
- [ ] Implement hourly aggregation support
- [ ] Add safety event alerting hooks

**Testing Requirements:**
- [ ] Create `tests/migrations/0015_create_environmental_metrics_test.rs`
- [ ] Test decibel range validations
- [ ] Test UV index constraints
- [ ] Test fall event recording
- [ ] Test aggregation queries

**Definition of Done:**
- All environmental fields implemented
- Safety alerting tested
- Apple Watch Series 8+ compatibility verified
- Performance benchmarks met
- Documentation includes safety protocols

---

### Stream 4: Mental Health and Advanced Metrics

---

#### Story 4.1: Create Mental Health Metrics Table

**Story Points:** 3  
**Assigned to:** Database Subagent  
**Priority:** Medium  

**Description:**
Implement mental_health_metrics for mindfulness and mood tracking (iOS 17+).

**Acceptance Criteria:**
- [ ] Create migration `migrations/0016_create_mental_health_metrics.sql` with:
  - Mindful session duration tracking
  - Mood valence (-1.0 to 1.0 scale)
  - Mood labels array field
  - Time in daylight tracking
- [ ] Add appropriate constraints for mood values
- [ ] Support array storage for mood labels

**Testing Requirements:**
- [ ] Create `tests/migrations/0016_create_mental_health_metrics_test.rs`
- [ ] Test mood valence range validation
- [ ] Test array field operations
- [ ] Test aggregation queries for mood trends
- [ ] Test iOS 17+ data import

**Definition of Done:**
- iOS 17+ compatibility verified
- Mood tracking validated
- Privacy considerations documented
- Sample data imports successfully
- API endpoints documented

---

#### Story 4.2: Create Mobility Metrics Table

**Story Points:** 5  
**Assigned to:** Database Subagent  
**Priority:** Low  

**Description:**
Implement mobility_metrics for advanced walking/running analysis (iOS 14+).

**Acceptance Criteria:**
- [ ] Create migration `migrations/0017_create_mobility_metrics.sql` with:
  - Walking speed and step length
  - Walking asymmetry percentage
  - Double support percentage
  - Six-minute walk test distance
  - Stair speed (up/down)
- [ ] Add appropriate biomechanical constraints
- [ ] Support high-frequency sampling

**Testing Requirements:**
- [ ] Create `tests/migrations/0017_create_mobility_metrics_test.rs`
- [ ] Test biomechanical range validations
- [ ] Test asymmetry calculations
- [ ] Test high-frequency data ingestion
- [ ] Test aggregation performance

**Definition of Done:**
- All 15 mobility fields implemented
- iOS 14+ compatibility verified
- Medical accuracy validated
- Performance targets met
- Clinical use cases documented

---

### Stream 5: Migration and Testing Infrastructure

---

#### Story 5.1: Create Data Migration Scripts

**Story Points:** 8  
**Assigned to:** Data Migration Subagent  
**Priority:** Critical  
**Depends on:** Stories 1.1, 2.1, 2.2  

**Description:**
Create comprehensive data migration scripts from old schema to new tables.

**Acceptance Criteria:**
- [ ] Create `scripts/migrate_activity_metrics.sql` for activity data
- [ ] Create field mapping logic for renamed fields
- [ ] Handle NULL value conversions appropriately
- [ ] Implement batch processing (8000 records/batch)
- [ ] Add progress tracking and resumability
- [ ] Create validation queries for data integrity

**Testing Requirements:**
- [ ] Create `tests/scripts/migrate_activity_metrics_test.rs`
- [ ] Test with production data sample (anonymized)
- [ ] Test batch processing performance
- [ ] Test resume after failure
- [ ] Test data integrity post-migration
- [ ] Test rollback procedures

**Definition of Done:**
- Zero data loss verified
- Migration time <4 hours for 100M records
- Rollback tested and documented
- Validation reports generated
- Production runbook created

---

#### Story 5.2: Update Rust Models and Handlers

**Story Points:** 13  
**Assigned to:** Backend Subagent  
**Priority:** Critical  
**Depends on:** All table creation stories  

**Description:**
Update Rust models, validation logic, and handlers for all new tables.

**Acceptance Criteria:**
- [ ] Update `src/models/health_metrics.rs` with new structs:
  - ActivityMetricsV2, NutritionMetrics, Symptoms
  - ReproductiveHealth, EnvironmentalMetrics, MentalHealthMetrics
- [ ] Update `src/handlers/ingest.rs` with routing for new types
- [ ] Implement validation with configurable thresholds
- [ ] Update `src/models/ios_models.rs` for Health Export mapping
- [ ] Add batch processing for new metric types

**Testing Requirements:**
- [ ] Create comprehensive test files:
  - `src/models/health_metrics_test.rs`
  - `src/handlers/ingest_test.rs`
  - `src/models/ios_models_test.rs`
- [ ] Test all validation thresholds
- [ ] Test batch processing with mixed metric types
- [ ] Test error handling and logging
- [ ] Integration tests with sample Health Export data

**Definition of Done:**
- All models implementing proper traits
- Validation coverage >95%
- All tests passing
- Performance benchmarks met
- API documentation generated

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