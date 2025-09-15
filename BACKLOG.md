


























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

## Additional Stories (Awaiting Master Story Completion)

*All other development blocked until STORY-MASTER-001 compilation issues resolved.*

## Legend
- 🔥 CRITICAL: Blocking compilation/deployment
- ⚠️ HIGH: Major functionality affected
- 📋 MEDIUM: Feature enhancement or optimization
- 💡 LOW: Nice-to-have improvements

---
*Last Updated: 2025-09-14*
*Total Active Stories: 1 (Master Story with 12 Sub-Stories)*