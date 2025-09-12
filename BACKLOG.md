# BACKLOG

# SCHEMA ALIGNMENT CRITICAL FIXES
*Created: September 12, 2025*
*Priority: Critical - Must be completed before production deployment*

## Component 1: Model Structure Updates (Critical Priority)





## Component 2: Field Mapping Fixes (High Priority)




## Component 3: Database Query Updates (High Priority)


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