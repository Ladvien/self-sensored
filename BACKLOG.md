# BACKLOG

# SCHEMA ALIGNMENT CRITICAL FIXES
*Created: September 12, 2025*
*Priority: Critical - Must be completed before production deployment*

## Component 1: Model Structure Updates (Critical Priority)





## Component 2: Field Mapping Fixes (High Priority)


**[SCHEMA-006] Fix Activity Metrics Field Name Mapping**
Priority: High
Points: 3
AC:
- Update all references from 'steps' to 'step_count' in validation logic
- Update all references from 'calories_burned' to 'active_energy_burned_kcal'
- Fix conversion methods between ActivityMetric and database structs
- Update field validation ranges for new field names
Dependencies: SCHEMA-002
Files: src/models/health_metrics.rs, src/models/db.rs


## Component 3: Database Query Updates (High Priority)

**[SCHEMA-008] Fix Batch Processor SQL Queries**
Priority: High
Points: 5
AC:
- Update INSERT INTO activity_metrics queries to use step_count, active_energy_burned_kcal
- Update INSERT INTO blood_pressure_metrics to use source_device instead of source
- Update INSERT INTO workouts to use avg_heart_rate instead of average_heart_rate
- Remove all references to activity_metrics_v2 table
- Remove all INSERT queries for deleted metric tables (nutrition_metrics, symptoms, etc.)
Dependencies: SCHEMA-002, SCHEMA-003, SCHEMA-005
Files: src/services/batch_processor.rs

**[SCHEMA-009] Fix Handler Query Field Names**
Priority: High
Points: 4
AC:
- Update query.rs SELECT statements to use step_count, active_energy_burned_kcal
- Fix export.rs field references for activity metrics
- Update workout queries to use started_at/ended_at field names
- Update all query responses to match simplified schema fields
Dependencies: SCHEMA-002, SCHEMA-003
Files: src/handlers/query.rs, src/handlers/export.rs

## Component 4: Database Structure Updates (Medium Priority)


## Component 5: Validation and Testing (Medium Priority)



**[SCHEMA-015] Update Integration Tests**
Priority: Medium
Points: 4
AC:
- Remove tests for deprecated metric types
- Update test payloads for simplified schema
- Fix field name assertions in existing tests
- Update test database setup for simplified schema
- Verify all tests pass with schema changes
Dependencies: SCHEMA-001, SCHEMA-002, SCHEMA-003
Files: tests/handlers/ingest_test.rs, tests/migrations/*.rs

## Component 6: Cleanup and Documentation (Low Priority)

**[SCHEMA-016] Clean Up Migration References**
Priority: Low
Points: 1
AC:
- Remove migration file references for deleted health metric tables
- Clean up migration test files for non-existent tables
- Update migration documentation
Dependencies: SCHEMA-001
Files: migrations/ (cleanup), tests/migrations/

**[SCHEMA-017] Update Configuration Documentation**
Priority: Low
Points: 1
AC:
- Update CLAUDE.md with simplified schema information
- Remove references to deprecated metric types in documentation
- Update field name examples in documentation
- Update environment variable documentation
Dependencies: All previous
Files: CLAUDE.md, .env.example

# STORY SUMMARY

**Total Stories Created: 17**

**Priority Breakdown:**
- Critical: 4 stories (17 points)
- High: 6 stories (18 points) 
- Medium: 5 stories (14 points)
- Low: 2 stories (2 points)

**Total Story Points: 51**

**Critical Path Dependencies:**
1. Component 1 (Model Structure Updates) - Must be completed first
2. Component 2 (Field Mapping Fixes) - Depends on Component 1
3. Component 3 (Database Query Updates) - Depends on Components 1 & 2
4. Components 4-6 can be worked in parallel after Component 3

**Files Most Affected:**
- src/models/health_metrics.rs (7 stories)
- src/services/batch_processor.rs (2 stories)
- src/handlers/*.rs (4 stories)
- src/models/db.rs (3 stories)

**Completion Order Recommendation:**
SCHEMA-001 � SCHEMA-002 � SCHEMA-003 � SCHEMA-004 � SCHEMA-005 � SCHEMA-006 � SCHEMA-007 � SCHEMA-008 � SCHEMA-009 � SCHEMA-010 � Remaining stories in parallel