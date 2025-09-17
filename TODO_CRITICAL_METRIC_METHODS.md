# CRITICAL: Missing Metric Type Methods Implementation

## Status: URGENT - Data Loss Prevention Fix in Progress

This documents the critical missing metric type methods that need to be implemented to prevent 52.9% data loss in the batch processor.

## Missing Insert Methods (8 methods needed)

1. `insert_metabolic_metrics()` - Process metabolic metrics (alcohol, insulin delivery)
2. `insert_environmental_metrics()` - Process environmental metrics (air quality, UV exposure)
3. `insert_audio_exposure_metrics()` - Process audio exposure metrics (hearing health)
4. `insert_safety_event_metrics()` - Process safety events (fall detection, emergency SOS)
5. `insert_mindfulness_metrics()` - Process mindfulness sessions (meditation, breathing)
6. `insert_mental_health_metrics()` - Process mental health metrics (state of mind, mood)
7. `insert_symptom_metrics()` - Process symptom tracking (headaches, pain, etc.)
8. `insert_hygiene_metrics()` - Process hygiene events (handwashing, toothbrushing)

## Missing Deduplication Methods (8 methods needed)

1. `deduplicate_metabolic_metrics()` - Deduplicate by user_id + recorded_at
2. `deduplicate_environmental_metrics()` - Deduplicate by user_id + recorded_at
3. `deduplicate_audio_exposure_metrics()` - Deduplicate by user_id + recorded_at
4. `deduplicate_safety_event_metrics()` - Deduplicate by user_id + recorded_at + event_type
5. `deduplicate_mindfulness_metrics()` - Deduplicate by user_id + session_start + session_end
6. `deduplicate_mental_health_metrics()` - Deduplicate by user_id + recorded_at
7. `deduplicate_symptom_metrics()` - Deduplicate by user_id + recorded_at + symptom_type
8. `deduplicate_hygiene_metrics()` - Deduplicate by user_id + recorded_at + event_type

## Missing Parallel Processing Support

Need to add parallel processing tasks for all 8 missing metric types in `process_parallel()` method.

## Missing DeduplicationStats Updates

Need to update remaining DeduplicationStats initializations throughout the file (6 locations found).

## Implementation Pattern

Each method should follow the existing pattern:
- Chunked insertion to respect PostgreSQL parameter limits
- Proper error handling with retry logic
- Transaction safety
- Comprehensive logging
- Metric collection for monitoring

## Priority: CRITICAL

This is preventing processing of 85,532+ metrics representing 52.9% data loss!

Affected metric types:
- Environmental: 84,432 metrics (49.4% of total)
- AudioExposure: 1,100 metrics (0.6% of total)
- SafetyEvent: ~100 metrics
- Mindfulness: ~500 metrics
- MentalHealth: ~200 metrics
- Symptom: ~150 metrics
- Hygiene: ~50 metrics
- Metabolic: Variable (insulin delivery, alcohol tracking)

## Current Status

✅ Structural fixes completed:
- Added missing fields to GroupedMetrics struct
- Added missing fields to DeduplicationStats struct
- Added missing match arms in group_metrics_by_type()
- Added missing processing logic in process_sequential()
- Added missing deduplication key structs

🚨 Remaining work:
- Stub out missing methods for immediate compilation
- Implement proper insert methods with database operations
- Implement proper deduplication methods
- Add parallel processing support
- Update remaining DeduplicationStats initializations
- Add comprehensive testing

## Database Tables Available

All corresponding database tables exist:
- metabolic_metrics
- environmental_metrics
- audio_exposure_metrics
- safety_event_metrics
- mindfulness_metrics
- mental_health_metrics
- symptom_metrics
- hygiene_metrics

## Next Steps

1. Create stub methods to prevent compilation errors and data loss
2. Implement proper database insert methods following existing patterns
3. Implement deduplication methods following existing patterns
4. Add parallel processing support
5. Update remaining DeduplicationStats references
6. Add comprehensive integration tests
7. Deploy and monitor data recovery