# Self-Sensored Health API - Product Backlog

## Schema Completion Sprint - Critical Missing Tables & Fields

*Generated from comprehensive DATA.md schema audit - 2025-09-14*

---

## EPIC: Complete Health Data Schema Implementation

### Priority P0 - Core Health Tracking (Sprint 1)

#### STORY-001: Add Body Measurements Table
**As a** health tracking user
**I want** to store my weight, BMI, body fat percentage, and height measurements
**So that** I can track my physical health metrics over time

**Acceptance Criteria:**
- [ ] Create `body_measurements` table with fields:
  - body_mass (weight in kg)
  - body_mass_index (calculated BMI)
  - body_fat_percentage
  - lean_body_mass (kg)
  - height (cm)
  - waist_circumference (cm)
- [ ] Add proper indexes for user_id + recorded_at queries
- [ ] Add validation constraints (reasonable ranges)
- [ ] Update ingestion handler to process body measurement data
- [ ] Add tests for body measurements storage and retrieval

**Technical Notes:**
- Reference HealthKit identifiers: HKQuantityTypeIdentifierBodyMass, HKQuantityTypeIdentifierBodyMassIndex, HKQuantityTypeIdentifierBodyFatPercentage, HKQuantityTypeIdentifierLeanBodyMass, HKQuantityTypeIdentifierHeight, HKQuantityTypeIdentifierWaistCircumference

---

#### STORY-002: Add Nutrition Metrics Table
**As a** user tracking my diet and nutrition
**I want** to store my food intake, macronutrients, vitamins, and hydration data
**So that** I can analyze my nutritional health and eating patterns

**Acceptance Criteria:**
- [ ] Create `nutrition_metrics` table with fields:
  - dietary_water (liters)
  - dietary_energy_consumed (calories)
  - dietary_carbohydrates (grams)
  - dietary_protein (grams)
  - dietary_fat_total (grams)
  - dietary_fat_saturated (grams)
  - dietary_cholesterol (mg)
  - dietary_sodium (mg)
  - dietary_fiber (grams)
  - dietary_sugar (grams)
  - dietary_calcium (mg)
  - dietary_iron (mg)
  - dietary_magnesium (mg)
  - dietary_potassium (mg)
  - dietary_caffeine (mg)
  - dietary_vitamin_a (mcg)
  - dietary_vitamin_c (mg)
  - dietary_vitamin_d (IU)
- [ ] Add composite indexes for efficient querying
- [ ] Implement nutritional data validation
- [ ] Update API endpoints for nutrition data ingestion
- [ ] Add comprehensive nutrition tests

**Technical Notes:**
- Covers all  supported dietary HealthKit types from DATA.md
- Consider daily aggregation patterns for nutritional analysis

---

#### STORY-003: Add Symptoms Tracking Table
**As a** user monitoring my health symptoms
**I want** to log various symptoms with severity and timing
**So that** I can track illness patterns and share data with healthcare providers

**Acceptance Criteria:**
- [ ] Create `symptoms` table with fields:
  - symptom_type (enum with 40+ symptom types)
  - severity (scale 1-5 or none/mild/moderate/severe/critical)
  - recorded_at (timestamp)
  - notes (optional text)
  - duration_minutes (optional)
- [ ] Create symptom_type enum including:
  - abdominal_cramps, bloating, breast_pain, headache, fatigue
  - nausea, dizziness, fever, coughing, shortness_of_breath
  - chest_tightness_or_pain, pelvic_pain, palpitations
  - Plus all other  supported symptoms from DATA.md
- [ ] Add indexes for symptom type and date range queries
- [ ] Implement symptom data ingestion endpoint
- [ ] Add symptom tracking tests and validation

**Technical Notes:**
- Map to HealthKit symptom category types
- Support batch symptom logging for illness tracking

---

### Priority P0 - Medical Data (Sprint 1 Continued)

#### STORY-004: Add Respiratory Metrics Table
**As a** user with respiratory conditions or fitness tracking
**I want** to store my respiratory rate, blood oxygen, and lung function data
**So that** I can monitor my breathing health and respiratory fitness

**Acceptance Criteria:**
- [ ] Create `respiratory_metrics` table with fields:
  - respiratory_rate (breaths per minute)
  - oxygen_saturation (SpO2 percentage)
  - forced_vital_capacity (liters)
  - forced_expiratory_volume_1 (FEV1 in liters)
  - peak_expiratory_flow_rate (L/min)
  - inhaler_usage (count/timestamp)
- [ ] Add validation for physiological ranges
- [ ] Create indexes for respiratory data queries
- [ ] Update ingestion pipeline for respiratory metrics
- [ ] Add respiratory data tests

**Technical Notes:**
- Critical for COVID-19 monitoring and respiratory health
- Supports pulse oximeter and spirometry device data

---

#### STORY-005: Add Blood & Metabolic Metrics Tables
**As a** user with diabetes or metabolic conditions
**I want** to store my blood glucose, insulin, and alcohol measurements
**So that** I can manage my metabolic health effectively

**Acceptance Criteria:**
- [ ] Create `blood_glucose_metrics` table:
  - blood_glucose_mg_dl (mg/dL)
  - measurement_context (fasting, post_meal, random, etc.)
  - medication_taken (boolean)
  - notes (optional)
- [ ] Create `metabolic_metrics` table:
  - blood_alcohol_content (BAC percentage)
  - insulin_delivery_units (insulin units)
  - delivery_method (injection, pump, etc.)
- [ ] Add proper medical data validation
- [ ] Implement secure handling of sensitive medical data
- [ ] Add metabolic data ingestion endpoints
- [ ] Create comprehensive medical data tests

**Technical Notes:**
- HIPAA compliance considerations for medical data
- Support CGM (Continuous Glucose Monitor) data streams

---

### Priority P1 - Extended Health Data (Sprint 2)

#### STORY-006: Add Temperature Metrics Table
**As a** user tracking my body temperature variations
**I want** to store body temperature, basal temperature, and environmental temperature data
**So that** I can monitor fever, fertility cycles, and thermal health

**Acceptance Criteria:**
- [ ] Create `temperature_metrics` table with fields:
  - body_temperature (celsius)
  - basal_body_temperature (celsius)
  - apple_sleeping_wrist_temperature (celsius)
  - water_temperature (celsius)
  - temperature_source (thermometer, wearable, etc.)
- [ ] Add temperature validation ranges
- [ ] Support fertility tracking temperature patterns
- [ ] Add temperature data ingestion
- [ ] Create temperature tracking tests

---

#### STORY-007: Add Reproductive Health Tables
**As a** user tracking reproductive health
**I want** to log menstrual cycles, fertility indicators, and reproductive events
**So that** I can monitor my reproductive health and fertility

**Acceptance Criteria:**
- [ ] Create `menstrual_health` table:
  - menstrual_flow (none, light, medium, heavy)
  - cycle_day (day number in cycle)
  - spotting (boolean)
  - cycle_regularity tracking
- [ ] Create `fertility_tracking` table:
  - cervical_mucus_quality (enum)
  - ovulation_test_result (positive, negative, high, peak)
  - sexual_activity (boolean with privacy)
  - pregnancy_test_result (positive, negative, indeterminate)
- [ ] Implement privacy-first data handling
- [ ] Add reproductive health ingestion endpoints
- [ ] Create reproductive health tests

**Technical Notes:**
- Privacy-sensitive data - ensure proper access controls
- Support fertility and period tracking apps integration

---

#### STORY-008: Add Environmental & Safety Metrics Tables
**As a** user concerned about environmental health exposure
**I want** to track audio exposure, UV exposure, daylight time, and safety events
**So that** I can monitor environmental health factors and safety incidents

**Acceptance Criteria:**
- [ ] Create `environmental_metrics` table:
  - uv_exposure_index
  - time_in_daylight_minutes
  - environmental_factors (air quality, etc.)
- [ ] Create `audio_exposure_metrics` table:
  - environmental_audio_exposure_db
  - headphone_audio_exposure_db
  - exposure_duration_minutes
  - audio_exposure_event (boolean for dangerous levels)
- [ ] Create `safety_events` table:
  - number_of_times_fallen (count)
  - fall_detected_at (timestamp)
  - emergency_contact_notified (boolean)
- [ ] Add environmental health ingestion
- [ ] Create environmental safety tests

---

### Priority P1 - Lifestyle & Mental Health (Sprint 2 Continued)

#### STORY-009: Add Mindfulness & Mental Health Tables
**As a** user practicing mindfulness and tracking mental health
**I want** to log meditation sessions and mood states
**So that** I can monitor my mental wellness and meditation practice

**Acceptance Criteria:**
- [ ] Create `mindfulness_sessions` table:
  - session_duration_minutes
  - meditation_type (breathing, body_scan, loving_kindness, etc.)
  - session_quality_rating (1-5)
  - notes (optional)
- [ ] Create `mental_health_metrics` table:
  - state_of_mind (iOS 17+ data)
  - mood_rating (scale)
  - anxiety_level (scale)
  - stress_level (scale)
  - notes (optional)
- [ ] Add mental health data privacy protections
- [ ] Implement mindfulness data ingestion
- [ ] Create mental health tracking tests

---

#### STORY-010: Add Hygiene Events Table
**As a** user tracking health behaviors and hygiene
**I want** to log handwashing and dental care events
**So that** I can monitor my health hygiene practices

**Acceptance Criteria:**
- [ ] Create `hygiene_events` table:
  - event_type (handwashing, toothbrushing)
  - event_occurred_at (timestamp)
  - duration_seconds (for timed events)
  - quality_rating (optional 1-5)
- [ ] Add hygiene behavior tracking
- [ ] Support public health hygiene monitoring
- [ ] Add hygiene event ingestion
- [ ] Create hygiene tracking tests

---

### Priority P2 - Enhanced Existing Tables (Sprint 3)

#### STORY-011: Extend Heart Rate Metrics Table
**As a** user with comprehensive heart monitoring devices
**I want** to store advanced heart rate data including HRV, recovery, and events
**So that** I can track detailed cardiovascular health metrics

**Acceptance Criteria:**
- [ ] Add missing fields to `heart_rate_metrics`:
  - walking_heart_rate_average
  - heart_rate_recovery_one_minute
  - atrial_fibrillation_burden_percentage
  - vo2_max_ml_kg_min
- [ ] Create `heart_rate_events` table:
  - event_type (high, low, irregular, afib)
  - event_occurred_at
  - heart_rate_at_event
  - event_duration_minutes
- [ ] Update heart rate ingestion for new fields
- [ ] Add advanced cardiovascular tests

---

#### STORY-012: Extend Activity Metrics Table
**As a** user with diverse activity tracking devices
**I want** to store specialized distance metrics and Apple Watch data
**So that** I can track all forms of physical activity comprehensively

**Acceptance Criteria:**
- [ ] Add missing fields to `activity_metrics`:
  - distance_cycling_meters
  - distance_swimming_meters
  - distance_wheelchair_meters
  - distance_downhill_snow_sports_meters
  - push_count (wheelchair pushes)
  - swimming_stroke_count
  - nike_fuel_points
  - apple_exercise_time_minutes
  - apple_stand_time_minutes
  - apple_move_time_minutes
  - apple_stand_hour_achieved (boolean)
- [ ] Update activity ingestion for specialized metrics
- [ ] Add comprehensive activity tracking tests

---

#### STORY-013: Extend Workouts Table with Full Workout Types
**As a** user doing diverse workout types
**I want** to track all 70+ workout types supported by HealthKit
**So that** I can accurately categorize and analyze all my exercise activities

**Acceptance Criteria:**
- [ ] Expand `workout_type` enum to include all HealthKit workout types:
  - Traditional: running, cycling, swimming, walking, strength_training
  - Sports: tennis, basketball, soccer, volleyball, baseball, etc.
  - Fitness: yoga, pilates, hiit, crosstraining, functionalstrength
  - Dance: dance, barre, etc.
  - Winter: skiing, snowboarding, skating
  - Water: surfing, rowing, sailing
  - Combat: boxing, martialarts, etc.
  - And 50+ more workout types from HealthKit
- [ ] Add `workout_routes` table for GPS tracking:
  - workout_id (FK)
  - route_points (JSON array of lat/lng/timestamp)
  - total_distance_meters
  - elevation_gain_meters
- [ ] Update workout ingestion for all workout types
- [ ] Add comprehensive workout tracking tests

---

#### STORY-014: Add User Characteristics Table
**As a** health tracking user
**I want** to store my static health characteristics
**So that** the system can provide personalized health insights and validation

**Acceptance Criteria:**
- [ ] Create `user_characteristics` table:
  - biological_sex (male, female, not_set)
  - blood_type (A_positive, A_negative, B_positive, etc.)
  - date_of_birth (date)
  - fitzpatrick_skin_type (1-6 for UV sensitivity)
  - wheelchair_use (boolean)
  - activity_move_mode (active_energy, move_time)
- [ ] Link characteristics to users table
- [ ] Use characteristics for health data validation
- [ ] Add user characteristics ingestion
- [ ] Create characteristics tests

---

## Technical Implementation Notes

### Database Constraints & Indexes
- All tables need user_id FK with CASCADE DELETE
- All timestamp fields need indexes for time-range queries
- Add composite indexes: (user_id, recorded_at DESC)
- Add check constraints for physiological value ranges
- Consider partitioning by month for high-volume metrics

### API Endpoints to Add
- `POST /v1/ingest/body-measurements`
- `POST /v1/ingest/nutrition`
- `POST /v1/ingest/symptoms`
- `POST /v1/ingest/respiratory`
- `POST /v1/ingest/blood-glucose`
- `POST /v1/ingest/reproductive-health`
- `POST /v1/ingest/environmental`
- `POST /v1/ingest/mindfulness`
- Plus corresponding GET endpoints for data retrieval

### Validation Requirements
- Implement medical-grade validation for all health metrics
- Add configurable validation thresholds via environment variables
- Ensure data integrity with proper error handling
- Support bulk validation for batch ingestion

### Testing Strategy
- Integration tests for each new table
- API endpoint tests for ingestion and retrieval
- Performance tests for large datasets
- Data validation tests for edge cases
- Privacy and security tests for sensitive data

### Migration Strategy
- Create migration scripts for each new table
- Ensure zero-downtime deployments
- Backup strategy for schema changes
- Rollback procedures for each migration

---

---

## EPIC: Complete API Handler Implementation

### Priority P0 - Core Handler Infrastructure (Sprint 1)

#### STORY-015: Add Respiratory Health API Handlers
**As a** user with respiratory conditions or fitness tracking needs
**I want** to submit and retrieve respiratory rate, SpO2, and lung function data via API
**So that** I can monitor my breathing health through the health data API

**Acceptance Criteria:**
- [ ] Create `respiratory_handler.rs` with endpoints:
  - `POST /api/v1/ingest/respiratory`
  - `GET /api/v1/data/respiratory`
- [ ] Add `RespiratoryMetric` struct to `health_metrics.rs`:
  ```rust
  pub struct RespiratoryMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub respiratory_rate: Option<i16>, // breaths per minute
      pub oxygen_saturation: Option<f64>, // SpO2 percentage
      pub forced_vital_capacity: Option<f64>, // liters
      pub forced_expiratory_volume_1: Option<f64>, // FEV1 liters
      pub peak_expiratory_flow_rate: Option<f64>, // L/min
      pub inhaler_usage: Option<i32>, // count
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing support in `ios_models.rs` for:
  - `respiratory_rate`, `oxygen_saturation`, `forced_vital_capacity`
  - `forced_expiratory_volume_1`, `peak_expiratory_flow_rate`, `inhaler_usage`
- [ ] Add respiratory data validation (medical ranges)
- [ ] Add routes to `main.rs`
- [ ] Add respiratory handler integration tests
- [ ] Update HealthMetric enum with Respiratory variant

**Technical Notes:**
- Maps to HealthKit: HKQuantityTypeIdentifierRespiratoryRate, HKQuantityTypeIdentifierOxygenSaturation, etc.
- Critical for COVID-19 monitoring and respiratory health

---

#### STORY-016: Add Body Measurements API Handlers
**As a** user tracking my physical health metrics
**I want** to submit and retrieve weight, BMI, height, and body composition data via API
**So that** I can monitor my physical health metrics through the API

**Acceptance Criteria:**
- [ ] Create `body_measurements_handler.rs` with endpoints:
  - `POST /api/v1/ingest/body-measurements`
  - `GET /api/v1/data/body-measurements`
- [ ] Add `BodyMeasurementMetric` struct to `health_metrics.rs`:
  ```rust
  pub struct BodyMeasurementMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub body_mass: Option<f64>, // weight in kg
      pub body_mass_index: Option<f64>, // BMI
      pub body_fat_percentage: Option<f64>,
      pub lean_body_mass: Option<f64>, // kg
      pub height: Option<f64>, // cm
      pub waist_circumference: Option<f64>, // cm
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing support for weight, height, BMI, body fat data
- [ ] Add body measurement validation (reasonable physical ranges)
- [ ] Add routes to `main.rs`
- [ ] Add body measurements integration tests
- [ ] Update HealthMetric enum with BodyMeasurement variant

**Technical Notes:**
- Maps to HealthKit: HKQuantityTypeIdentifierBodyMass, HKQuantityTypeIdentifierHeight, etc.
- Essential for fitness and weight management tracking

---

#### STORY-017: Add Symptoms Tracking API Handlers
**As a** user monitoring my health symptoms
**I want** to submit and retrieve symptom data with severity levels via API
**So that** I can track illness patterns and health symptoms

**Acceptance Criteria:**
- [ ] Create `symptoms_handler.rs` with endpoints:
  - `POST /api/v1/ingest/symptoms`
  - `GET /api/v1/data/symptoms`
- [ ] Add `SymptomMetric` struct and enums to `health_metrics.rs`:
  ```rust
  pub struct SymptomMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub symptom_type: SymptomType,
      pub severity: SymptomSeverity,
      pub duration_minutes: Option<i32>,
      pub notes: Option<String>,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }

  pub enum SymptomType {
      AbdominalCramps, Bloating, BreastPain, Headache, Fatigue,
      Nausea, Dizziness, Fever, Coughing, ShortnessOfBreath,
      ChestTightnessOrPain, PelvicPain, Palpitations,
      // ... 30+ more symptom types from DATA.md
  }

  pub enum SymptomSeverity {
      None, Mild, Moderate, Severe, Critical,
  }
  ```
- [ ] Add iOS parsing support for all 40+ symptom types
- [ ] Add symptom severity parsing from iOS data
- [ ] Add routes to `main.rs`
- [ ] Add symptoms tracking integration tests
- [ ] Update HealthMetric enum with Symptom variant

**Technical Notes:**
- Maps to HealthKit symptom category types
- Support batch symptom logging for illness tracking

---

#### STORY-018: Add Temperature Metrics API Handlers
**As a** user tracking body temperature variations
**I want** to submit and retrieve temperature data via API
**So that** I can monitor fever, fertility cycles, and thermal health

**Acceptance Criteria:**
- [ ] Create `temperature_handler.rs` with endpoints:
  - `POST /api/v1/ingest/temperature`
  - `GET /api/v1/data/temperature`
- [ ] Add `TemperatureMetric` struct to `health_metrics.rs`:
  ```rust
  pub struct TemperatureMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub body_temperature: Option<f64>, // celsius
      pub basal_body_temperature: Option<f64>, // celsius
      pub apple_sleeping_wrist_temperature: Option<f64>, // celsius
      pub water_temperature: Option<f64>, // celsius
      pub temperature_source: Option<String>, // thermometer type
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing for temperature data types
- [ ] Add temperature validation (medical ranges)
- [ ] Add routes to `main.rs`
- [ ] Add temperature tracking integration tests
- [ ] Update HealthMetric enum with Temperature variant

**Technical Notes:**
- Maps to HealthKit: HKQuantityTypeIdentifierBodyTemperature, HKQuantityTypeIdentifierBasalBodyTemperature, etc.
- Critical for fertility tracking and fever monitoring

---

### Priority P1 - Advanced Health Data (Sprint 2)

#### STORY-019: Add Nutrition Data API Handlers
**As a** user tracking my diet and nutrition intake
**I want** to submit and retrieve comprehensive nutritional data via API
**So that** I can analyze my food intake and nutritional health

**Acceptance Criteria:**
- [ ] Create `nutrition_handler.rs` with endpoints:
  - `POST /api/v1/ingest/nutrition`
  - `GET /api/v1/data/nutrition`
  - `GET /api/v1/data/hydration`
- [ ] Add `NutritionMetric` struct with 20+ nutritional fields:
  ```rust
  pub struct NutritionMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,

      // Hydration
      pub dietary_water: Option<f64>, // liters
      pub dietary_caffeine: Option<f64>, // mg

      // Macronutrients
      pub dietary_energy_consumed: Option<f64>, // calories
      pub dietary_carbohydrates: Option<f64>, // grams
      pub dietary_protein: Option<f64>, // grams
      pub dietary_fat_total: Option<f64>, // grams
      pub dietary_fiber: Option<f64>, // grams
      pub dietary_sugar: Option<f64>, // grams

      // Minerals & Vitamins (20+ additional fields)
      pub dietary_calcium: Option<f64>, // mg
      pub dietary_iron: Option<f64>, // mg
      // ... etc for all vitamins/minerals from DATA.md

      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing for all dietary HealthKit types
- [ ] Add nutritional validation (reasonable daily intake ranges)
- [ ] Add routes to `main.rs`
- [ ] Add nutrition tracking integration tests
- [ ] Update HealthMetric enum with Nutrition variant

**Technical Notes:**
- Covers all  supported dietary HealthKit types from DATA.md
- Consider daily aggregation patterns for nutritional analysis

---

#### STORY-020: Add Blood Glucose & Metabolic API Handlers
**As a** user managing diabetes and metabolic health
**I want** to submit and retrieve blood glucose, insulin, and metabolic data via API
**So that** I can manage my metabolic health effectively

**Acceptance Criteria:**
- [ ] Create `metabolic_handler.rs` with endpoints:
  - `POST /api/v1/ingest/blood-glucose`
  - `POST /api/v1/ingest/metabolic`
  - `GET /api/v1/data/blood-glucose`
  - `GET /api/v1/data/metabolic`
- [ ] Add metabolic metric structs:
  ```rust
  pub struct BloodGlucoseMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub blood_glucose_mg_dl: f64,
      pub measurement_context: GlucoseContext, // fasting, post_meal, etc
      pub medication_taken: bool,
      pub notes: Option<String>,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }

  pub struct MetabolicMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub blood_alcohol_content: Option<f64>,
      pub insulin_delivery_units: Option<f64>,
      pub delivery_method: Option<String>,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing for metabolic data
- [ ] Add medical-grade validation for blood glucose data
- [ ] Add secure handling for sensitive medical data
- [ ] Add routes to `main.rs`
- [ ] Add metabolic data integration tests
- [ ] Update HealthMetric enum with BloodGlucose and Metabolic variants

**Technical Notes:**
- HIPAA compliance considerations for medical data
- Support CGM (Continuous Glucose Monitor) data streams

---

### Priority P1 - Specialized Health Data (Sprint 2 Continued)

#### STORY-021: Add Reproductive Health API Handlers
**As a** user tracking reproductive health
**I want** to submit and retrieve menstrual cycle and fertility data via API
**So that** I can monitor reproductive health and fertility patterns

**Acceptance Criteria:**
- [ ] Create `reproductive_health_handler.rs` with endpoints:
  - `POST /api/v1/ingest/reproductive-health`
  - `GET /api/v1/data/menstrual`
  - `GET /api/v1/data/fertility`
- [ ] Add reproductive health metric structs:
  ```rust
  pub struct MenstrualMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub menstrual_flow: MenstrualFlow,
      pub spotting: bool,
      pub cycle_day: Option<i16>,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }

  pub struct FertilityMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub cervical_mucus_quality: CervicalMucusQuality,
      pub ovulation_test_result: OvulationTestResult,
      pub sexual_activity: bool, // privacy-protected
      pub pregnancy_test_result: PregnancyTestResult,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add reproductive health enums (flow levels, test results, etc.)
- [ ] Add iOS parsing for reproductive health data
- [ ] Implement privacy-first data handling
- [ ] Add routes to `main.rs`
- [ ] Add reproductive health integration tests
- [ ] Update HealthMetric enum with Menstrual and Fertility variants

**Technical Notes:**
- Privacy-sensitive data - ensure proper access controls
- Support fertility and period tracking apps integration

---

### Priority P2 - Environmental & Lifestyle (Sprint 3)

#### STORY-022: Add Environmental & Safety API Handlers
**As a** user monitoring environmental health factors
**I want** to submit and retrieve environmental exposure and safety data via API
**So that** I can track environmental health impacts and safety events

**Acceptance Criteria:**
- [ ] Create `environmental_handler.rs` with endpoints:
  - `POST /api/v1/ingest/environmental`
  - `POST /api/v1/ingest/audio-exposure`
  - `POST /api/v1/ingest/safety-events`
  - `GET /api/v1/data/environmental`
- [ ] Add environmental metric structs:
  ```rust
  pub struct EnvironmentalMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub uv_exposure_index: Option<f64>,
      pub time_in_daylight_minutes: Option<i32>,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }

  pub struct AudioExposureMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub environmental_audio_exposure_db: Option<f64>,
      pub headphone_audio_exposure_db: Option<f64>,
      pub exposure_duration_minutes: i32,
      pub audio_exposure_event: bool, // dangerous level
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing for environmental data
- [ ] Add environmental health validation
- [ ] Add routes to `main.rs`
- [ ] Add environmental tracking integration tests
- [ ] Update HealthMetric enum with Environmental variants

---

#### STORY-023: Add Mindfulness & Mental Health API Handlers
**As a** user practicing mindfulness and tracking mental wellness
**I want** to submit and retrieve meditation and mental health data via API
**So that** I can monitor mental wellness through the API

**Acceptance Criteria:**
- [ ] Create `mindfulness_handler.rs` with endpoints:
  - `POST /api/v1/ingest/mindfulness`
  - `POST /api/v1/ingest/mental-health`
  - `GET /api/v1/data/mindfulness`
  - `GET /api/v1/data/mental-health`
- [ ] Add mindfulness metric structs:
  ```rust
  pub struct MindfulnessMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub session_duration_minutes: i32,
      pub meditation_type: MeditationType,
      pub session_quality_rating: Option<i16>, // 1-5
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }

  pub struct MentalHealthMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub state_of_mind: StateOfMind, // iOS 17+ feature
      pub mood_rating: Option<i16>,
      pub anxiety_level: Option<i16>,
      pub stress_level: Option<i16>,
      pub notes: Option<String>,
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing for mindfulness and mental health data
- [ ] Add mental health data privacy protections
- [ ] Add routes to `main.rs`
- [ ] Add mindfulness tracking integration tests
- [ ] Update HealthMetric enum with Mindfulness variants

---

#### STORY-024: Add Hygiene Events API Handlers
**As a** user tracking health behaviors and hygiene habits
**I want** to submit and retrieve hygiene event data via API
**So that** I can monitor hygiene practices through the API

**Acceptance Criteria:**
- [ ] Create `hygiene_handler.rs` with endpoints:
  - `POST /api/v1/ingest/hygiene`
  - `GET /api/v1/data/hygiene`
- [ ] Add hygiene metric struct:
  ```rust
  pub struct HygieneMetric {
      pub id: uuid::Uuid,
      pub user_id: uuid::Uuid,
      pub recorded_at: DateTime<Utc>,
      pub event_type: HygieneEventType, // handwashing, toothbrushing
      pub duration_seconds: Option<i32>,
      pub quality_rating: Option<i16>, // 1-5
      pub source_device: Option<String>,
      pub created_at: DateTime<Utc>,
  }
  ```
- [ ] Add iOS parsing for hygiene events
- [ ] Add hygiene behavior validation
- [ ] Add routes to `main.rs`
- [ ] Add hygiene tracking integration tests
- [ ] Update HealthMetric enum with Hygiene variant

---

### Priority P2 - Handler Infrastructure Improvements (Sprint 3)

#### STORY-025: Refactor iOS Parser for Comprehensive HealthKit Support
**As a** developer maintaining the iOS data parser
**I want** to support all 200+ HealthKit data types with proper parsing
**So that** the API can handle the full range of iOS Auto Health Export data

**Acceptance Criteria:**
- [ ] Expand `ios_models.rs` parsing to handle all HealthKit identifiers from DATA.md
- [ ] Add specialized parsers for complex data types (symptoms with severity, etc.)
- [ ] Add metadata extraction from iOS JSON for device information
- [ ] Add validation for medical data ranges during parsing
- [ ] Add comprehensive parsing error handling and logging
- [ ] Add parser unit tests for all supported HealthKit types
- [ ] Update parser to handle iOS JSON variations and edge cases

**Technical Notes:**
- Current parser only handles ~15 data types out of 200+ supported
- Need pattern matching for all HealthKit identifier variations
- Critical for comprehensive iOS Auto Health Export integration

---

#### STORY-026: Add Specialized Validation per Health Metric Type
**As a** developer ensuring data quality
**I want** each health metric type to have appropriate medical-grade validation
**So that** the API maintains high data quality standards

**Acceptance Criteria:**
- [ ] Add medical-range validation for each health metric type
- [ ] Add configurable validation thresholds via environment variables
- [ ] Add validation context awareness (age, gender considerations)
- [ ] Add comprehensive validation error messages
- [ ] Add validation unit tests for all metric types
- [ ] Add validation performance benchmarks
- [ ] Document validation ranges and medical rationale

**Technical Notes:**
- Extend existing ValidationConfig for all new metric types
- Consider medical literature for appropriate ranges
- Support validation customization for different user populations

---

## Updated Summary

---

## EPIC: Complete Batch Processing Infrastructure

### Priority P0 - Critical Batch Processing Gaps (Sprint 1)

#### STORY-027: Add Blood Glucose Batch Processing for CGM Data Streams
**As a** developer supporting diabetes management
**I want** batch processing for continuous glucose monitor (CGM) data streams
**So that** the API can handle high-frequency medical device data (288 readings/day per user)

**Acceptance Criteria:**
- [ ] Add `BloodGlucoseMetric` batch processing to `BatchProcessor`
- [ ] Implement chunking for blood glucose data (chunk size: ~10,000 records)
- [ ] Add specialized deduplication: user_id + recorded_at + glucose_source
- [ ] Add medical-critical validation ranges (70-180 mg/dL normal)
- [ ] Implement atomic insulin + glucose pairing transaction logic
- [ ] Add CGM-specific high-frequency data handling
- [ ] Add blood glucose batch processing tests with large datasets
- [ ] Add monitoring for medical data processing failures

**Technical Notes:**
- CGM generates 1 reading every 5 minutes = 288 readings/day
- Requires ACID compliance for insulin delivery pairing
- Critical for diabetes management - zero data loss tolerance

**Definition of Done:**
- [ ] Can process 10,000+ glucose readings in single batch
- [ ] Maintains transaction integrity for medical data
- [ ] Validation prevents dangerous glucose value acceptance
- [ ] Performance benchmarks meet medical device requirements

---

#### STORY-028: Add Respiratory Metrics Batch Processing
**As a** developer supporting respiratory health monitoring
**I want** batch processing for SpO2, respiratory rate, and lung function data
**So that** the API can handle respiratory health tracking from pulse oximeters and spirometry devices

**Acceptance Criteria:**
- [ ] Add `RespiratoryMetric` struct to `health_metrics.rs`
- [ ] Implement respiratory metrics batch processing in `BatchProcessor`
- [ ] Add chunking strategy (chunk size: ~7,000 records, 7 params per record)
- [ ] Add respiratory deduplication: user_id + recorded_at + measurement_type
- [ ] Add medical validation ranges:
  - SpO2: 90-100% normal, <90% critical alert
  - Respiratory rate: 12-20 breaths/minute normal
  - FEV1, FVC: Medical reference ranges by age/gender
- [ ] Add inhaler usage tracking support
- [ ] Add respiratory batch processing integration tests
- [ ] Update `GroupedMetrics` and deduplication logic

**Technical Notes:**
- Maps to HealthKit: HKQuantityTypeIdentifierRespiratoryRate, HKQuantityTypeIdentifierOxygenSaturation
- Critical for COVID-19 monitoring and respiratory health
- Pulse oximeter data can be high-frequency (continuous monitoring)

**Definition of Done:**
- [ ] Can batch process respiratory data from multiple device types
- [ ] Validates medical ranges and alerts on critical values
- [ ] Supports both manual and continuous respiratory monitoring
- [ ] Performance handles continuous SpO2 monitoring streams

---

#### STORY-029: Add Body Measurements Batch Processing
**As a** developer supporting fitness and health tracking
**I want** batch processing for weight, BMI, body composition, and physical measurements
**So that** the API can handle daily measurements from smart scales and body composition devices

**Acceptance Criteria:**
- [ ] Add `BodyMeasurementMetric` struct to `health_metrics.rs`
- [ ] Implement body measurements batch processing in `BatchProcessor`
- [ ] Add chunking strategy (chunk size: ~8,000 records, 8 params per record)
- [ ] Add body measurement deduplication: user_id + recorded_at + measurement_type
- [ ] Add validation ranges:
  - BMI: 15-50 range, calculate consistency checks
  - Body fat: 3-50% range by gender
  - Weight: 20-500 kg range
  - Height: 50-250 cm range
- [ ] Add BMI calculation validation (weight/height² consistency)
- [ ] Add body measurements batch processing tests
- [ ] Update existing deduplication infrastructure

**Technical Notes:**
- Maps to HealthKit: HKQuantityTypeIdentifierBodyMass, HKQuantityTypeIdentifierBodyMassIndex, etc.
- Essential for fitness and weight management tracking
- Smart scale data typically includes multiple measurements per reading

**Definition of Done:**
- [ ] Can batch process multi-metric body composition data
- [ ] BMI calculation validation prevents inconsistent data
- [ ] Supports both manual entry and smart device integration
- [ ] Handles historical body measurement imports

---

### Priority P0 - Complex Data Processing (Sprint 1 Continued)

#### STORY-030: Add Symptoms Tracking Batch Processing
**As a** developer supporting illness and symptom tracking
**I want** batch processing for 40+ symptom types with severity levels and episode grouping
**So that** users can batch log illness episodes and symptom patterns

**Acceptance Criteria:**
- [ ] Add `SymptomMetric` struct with 40+ symptom types enum
- [ ] Add `SymptomType` enum mapping all HealthKit symptom categories:
  - AbdominalCramps, Bloating, BreastPain, Headache, Fatigue
  - Nausea, Dizziness, Fever, Coughing, ShortnessOfBreath
  - ChestTightnessOrPain, PelvicPain, Palpitations, etc.
- [ ] Add `SymptomSeverity` enum: None, Mild, Moderate, Severe, Critical
- [ ] Implement symptoms batch processing with episode transaction support
- [ ] Add multi-symptom deduplication: user_id + recorded_at + symptom_type
- [ ] Add episode transaction grouping (multiple symptoms per illness atomically)
- [ ] Add symptom severity validation and correlation checks
- [ ] Add symptoms batch processing comprehensive tests

**Technical Notes:**
- Maps to all HealthKit symptom category types from DATA.md
- Support batch symptom logging for illness tracking
- Episode-based transactions critical for medical accuracy

**Definition of Done:**
- [ ] Can process 40+ different symptom types in batches
- [ ] Maintains episode integrity (all symptoms for illness stored atomically)
- [ ] Severity validation prevents inconsistent symptom logging
- [ ] Supports bulk illness episode import/export

---

### Priority P1 - High-Volume Data Processing (Sprint 2)

#### STORY-031: Add Nutrition Data Batch Processing with Meal Grouping
**As a** developer supporting comprehensive nutrition tracking
**I want** batch processing for 25+ nutritional fields with meal-based transaction support
**So that** the API can handle complex nutrition logging with atomic meal storage

**Acceptance Criteria:**
- [ ] Add `NutritionMetric` struct with 25+ nutritional fields:
  - Hydration: dietary_water, dietary_caffeine
  - Macros: calories, carbs, protein, fat, fiber, sugar
  - Minerals: calcium, iron, magnesium, potassium, sodium
  - Vitamins: A, C, D, B6, B12, etc.
- [ ] Implement nutrition batch processing (chunk size: ~2,500 records, 25+ params)
- [ ] Add complex deduplication: user_id + recorded_at + nutrient_type
- [ ] Add meal-based transaction grouping (atomic meal component storage)
- [ ] Add nutritional validation ranges (daily intake limits)
- [ ] Add support for multiple nutrients per meal processing
- [ ] Add nutrition batch processing integration tests with meal scenarios
- [ ] Update iOS parser for all dietary HealthKit types

**Technical Notes:**
- Covers all ✅ supported dietary HealthKit types from DATA.md
- 20+ nutrients per meal × 3+ meals = 60+ records/day per user
- Meal atomicity critical for nutritional analysis accuracy

**Definition of Done:**
- [ ] Can process complete meals with all nutrients atomically
- [ ] Validates daily nutritional intake limits
- [ ] Supports bulk nutrition data import from food tracking apps
- [ ] Performance handles high-volume nutrition logging users

---

#### STORY-032: Add Temperature Metrics Batch Processing
**As a** developer supporting fertility and fever tracking
**I want** batch processing for multiple temperature sources and continuous monitoring
**So that** the API can handle basal body temperature patterns and fever monitoring

**Acceptance Criteria:**
- [ ] Add `TemperatureMetric` struct with temperature source support
- [ ] Add temperature types: body, basal_body, wrist_temperature, water
- [ ] Implement temperature batch processing (chunk size: ~10,000, 6 params per record)
- [ ] Add multi-source deduplication: user_id + recorded_at + temperature_source
- [ ] Add temperature validation ranges (36-42°C body temp, fever thresholds)
- [ ] Add fertility cycle pattern validation for basal temperature
- [ ] Add continuous temperature monitoring support (Apple Watch wrist temp)
- [ ] Add temperature batch processing tests with fertility scenarios

**Technical Notes:**
- Maps to HealthKit: HKQuantityTypeIdentifierBodyTemperature, etc.
- Critical for fertility tracking (basal temp patterns) and fever monitoring
- Apple Watch provides continuous wrist temperature during sleep

**Definition of Done:**
- [ ] Can process multiple temperature sources simultaneously
- [ ] Validates fertility temperature patterns for consistency
- [ ] Supports continuous temperature monitoring streams
- [ ] Handles historical temperature data bulk imports

---

### Priority P1 - Specialized Health Data (Sprint 2 Continued)

#### STORY-033: Add Reproductive Health Batch Processing with Privacy Controls
**As a** developer supporting women's health tracking
**I want** batch processing for menstrual cycle and fertility data with privacy protections
**So that** the API can handle reproductive health tracking with appropriate data security

**Acceptance Criteria:**
- [ ] Add `MenstrualMetric` and `FertilityMetric` structs
- [ ] Add menstrual flow enum: None, Light, Medium, Heavy
- [ ] Add fertility tracking fields: cervical mucus, ovulation tests, pregnancy tests
- [ ] Implement reproductive health batch processing with privacy controls
- [ ] Add cycle-aware deduplication: user_id + cycle_day + metric_type
- [ ] Add cycle consistency validation (flow patterns, timing)
- [ ] Add privacy-first data handling (encrypted sensitive fields)
- [ ] Add reproductive health batch processing tests with cycle scenarios
- [ ] Add access control validation for sensitive reproductive data

**Technical Notes:**
- Privacy-sensitive data - ensure proper access controls
- Support fertility and period tracking apps integration
- Cycle-based validation critical for medical accuracy

**Definition of Done:**
- [ ] Can process complete menstrual cycles with privacy protection
- [ ] Validates cycle patterns for medical consistency
- [ ] Supports bulk fertility tracking data import
- [ ] Privacy controls prevent unauthorized reproductive data access

---

### Priority P2 - Advanced Batch Processing Architecture (Sprint 3)

#### STORY-034: Implement Generic Batch Processing Framework
**As a** developer extending batch processing capabilities
**I want** a generic framework for registering new health metric types
**So that** adding new batch processing support doesn't require architectural changes

**Acceptance Criteria:**
- [ ] Design generic `MetricBatchProcessor<T>` trait
- [ ] Add dynamic metric type registration system
- [ ] Implement configurable chunking strategies per metric type
- [ ] Add pluggable deduplication strategy system:
  ```rust
  pub enum DeduplicationStrategy {
      Simple(SimpleKey),           // user_id + timestamp
      Composite(CompositeKey),     // multi-field keys
      Conditional(ConditionalKey), // context-aware dedup
  }
  ```
- [ ] Add generic validation framework integration
- [ ] Add metric type-specific monitoring and alerting
- [ ] Add comprehensive framework tests with multiple metric types
- [ ] Update existing batch processors to use generic framework

**Technical Notes:**
- Future-proofs batch processing for new health metrics
- Reduces code duplication across metric type processors
- Enables plugin architecture for third-party health metrics

**Definition of Done:**
- [ ] New metric types can be added with minimal code changes
- [ ] Generic framework maintains same performance as specialized processors
- [ ] Supports all existing deduplication and validation patterns
- [ ] Documentation provides clear guide for adding new metric types

---

#### STORY-035: Add High-Frequency Data Stream Processing
**As a** developer supporting real-time medical devices
**I want** streaming ingestion capabilities for high-frequency health data
**So that** the API can handle continuous monitoring devices without memory limits

**Acceptance Criteria:**
- [ ] Design streaming ingestion architecture for real-time data
- [ ] Add `process_stream<T>(&self, stream: impl Stream<Item = T>)` method
- [ ] Implement memory-efficient stream processing (bounded buffers)
- [ ] Add backpressure handling for high-volume streams
- [ ] Add stream-specific monitoring and error recovery
- [ ] Add support for CGM data streams (288 readings/day per user)
- [ ] Add continuous heart rate stream processing (1/second)
- [ ] Add comprehensive stream processing tests with realistic data volumes

**Technical Notes:**
- Critical for continuous glucose monitors (CGM) and cardiac monitors
- Must handle thousands of users with continuous data streams
- Memory efficiency crucial for production scalability

**Definition of Done:**
- [ ] Can process continuous data streams without memory growth
- [ ] Handles backpressure gracefully under high load
- [ ] Maintains data integrity under stream processing failures
- [ ] Performance benchmarks validate production readiness

---

#### STORY-036: Add Cross-Metric Transaction Support
**As a** developer ensuring medical data integrity
**I want** transaction support for related health metrics that must be stored together
**So that** medical data maintains referential integrity across metric types

**Acceptance Criteria:**
- [ ] Design `RelatedMetricSet` transaction system
- [ ] Add `process_related_metrics(&self, metrics: RelatedMetricSet)` method
- [ ] Implement atomic storage for related metrics:
  - Insulin delivery + blood glucose readings
  - Symptom episodes (multiple symptoms per illness)
  - Workout + heart rate + GPS route data
  - Meal components (multiple nutrients per meal)
- [ ] Add cross-metric validation (consistency between related data)
- [ ] Add transaction rollback for partial failures
- [ ] Add comprehensive cross-metric transaction tests
- [ ] Add performance benchmarks for complex transactions

**Technical Notes:**
- Medical data integrity requires ACID compliance across metric types
- Critical for diabetes management (insulin + glucose pairing)
- Complex workout data (multiple metrics per exercise session)

**Definition of Done:**
- [ ] Related metrics are always stored atomically or not at all
- [ ] Cross-metric validation prevents inconsistent medical data
- [ ] Transaction performance meets medical device requirements
- [ ] Rollback functionality maintains database consistency

---

## Updated Summary

*Total Stories: 36 (14 Schema + 12 API Handlers + 10 Batch Processing)*
*Estimated Story Points: ~280-340*
*Estimated Sprints: 6-7*
*Priority: CRITICAL for production readiness*
