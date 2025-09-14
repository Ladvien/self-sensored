-- Add missing health metrics tables for complete iOS Health data support

-- Blood Glucose Metrics (Medical-Critical CGM Data)
CREATE TABLE IF NOT EXISTS blood_glucose_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,
    blood_glucose_mg_dl DOUBLE PRECISION NOT NULL,
    measurement_context VARCHAR(50), -- 'fasting', 'post_meal', 'random', etc.
    medication_taken BOOLEAN,
    insulin_delivery_units DOUBLE PRECISION,
    glucose_source VARCHAR(255), -- CGM device identifier
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at, glucose_source)
);

-- 1. Nutrition Metrics (Macronutrients)
CREATE TABLE IF NOT EXISTS nutrition_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Macronutrients
    calories DOUBLE PRECISION,
    protein_grams DOUBLE PRECISION,
    carbohydrates_grams DOUBLE PRECISION,
    fiber_grams DOUBLE PRECISION,
    sugar_grams DOUBLE PRECISION,
    fat_total_grams DOUBLE PRECISION,
    fat_saturated_grams DOUBLE PRECISION,
    fat_monounsaturated_grams DOUBLE PRECISION,
    fat_polyunsaturated_grams DOUBLE PRECISION,
    cholesterol_mg DOUBLE PRECISION,
    sodium_mg DOUBLE PRECISION,
    water_ml DOUBLE PRECISION,
    caffeine_mg DOUBLE PRECISION,
    alcohol_grams DOUBLE PRECISION,

    -- Metadata
    meal_type VARCHAR(50), -- breakfast, lunch, dinner, snack
    food_item VARCHAR(500),
    brand VARCHAR(255),
    barcode VARCHAR(100),
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at, meal_type, food_item)
);

-- 2. Micronutrients (Vitamins & Minerals)
CREATE TABLE IF NOT EXISTS micronutrients (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Vitamins (in appropriate units)
    vitamin_a_mcg DOUBLE PRECISION,
    vitamin_c_mg DOUBLE PRECISION,
    vitamin_d_mcg DOUBLE PRECISION,
    vitamin_e_mg DOUBLE PRECISION,
    vitamin_k_mcg DOUBLE PRECISION,
    vitamin_b1_thiamin_mg DOUBLE PRECISION,
    vitamin_b2_riboflavin_mg DOUBLE PRECISION,
    vitamin_b3_niacin_mg DOUBLE PRECISION,
    vitamin_b5_pantothenic_acid_mg DOUBLE PRECISION,
    vitamin_b6_mg DOUBLE PRECISION,
    vitamin_b7_biotin_mcg DOUBLE PRECISION,
    vitamin_b9_folate_mcg DOUBLE PRECISION,
    vitamin_b12_mcg DOUBLE PRECISION,

    -- Minerals
    calcium_mg DOUBLE PRECISION,
    iron_mg DOUBLE PRECISION,
    magnesium_mg DOUBLE PRECISION,
    phosphorus_mg DOUBLE PRECISION,
    potassium_mg DOUBLE PRECISION,
    zinc_mg DOUBLE PRECISION,
    copper_mg DOUBLE PRECISION,
    manganese_mg DOUBLE PRECISION,
    selenium_mcg DOUBLE PRECISION,
    iodine_mcg DOUBLE PRECISION,
    chromium_mcg DOUBLE PRECISION,
    molybdenum_mcg DOUBLE PRECISION,

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at)
);

-- 3. Body Measurements
CREATE TABLE IF NOT EXISTS body_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Weight & Composition
    body_weight_kg DOUBLE PRECISION,
    body_mass_index DOUBLE PRECISION,
    body_fat_percentage DOUBLE PRECISION,
    lean_body_mass_kg DOUBLE PRECISION,

    -- Measurements
    waist_circumference_cm DOUBLE PRECISION,
    hip_circumference_cm DOUBLE PRECISION,
    chest_circumference_cm DOUBLE PRECISION,
    arm_circumference_cm DOUBLE PRECISION,
    thigh_circumference_cm DOUBLE PRECISION,

    -- Body Temperature
    body_temperature_celsius DOUBLE PRECISION,
    basal_body_temperature_celsius DOUBLE PRECISION,

    -- Metadata
    measurement_source VARCHAR(100), -- manual, smart_scale, apple_watch
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at, measurement_source)
);

-- 4. Respiratory & Cardiovascular Metrics
CREATE TABLE IF NOT EXISTS respiratory_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Respiratory
    respiratory_rate_breaths_per_min DOUBLE PRECISION,
    forced_vital_capacity_ml DOUBLE PRECISION,
    forced_expiratory_volume_ml DOUBLE PRECISION,
    peak_expiratory_flow_rate_ml_per_min DOUBLE PRECISION,
    oxygen_saturation_percent DOUBLE PRECISION,
    vo2_max_ml_per_kg_per_min DOUBLE PRECISION,

    -- Additional Cardiovascular
    peripheral_perfusion_index DOUBLE PRECISION,
    blood_glucose_mg_per_dl DOUBLE PRECISION,
    blood_alcohol_content_percent DOUBLE PRECISION,

    -- Metadata
    activity_context VARCHAR(100), -- resting, post_exercise, during_sleep
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at, activity_context)
);

-- 5. Environmental & Exposure Metrics
CREATE TABLE IF NOT EXISTS environmental_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Sound Exposure
    environmental_audio_exposure_db DOUBLE PRECISION,
    headphone_audio_exposure_db DOUBLE PRECISION,

    -- UV Exposure
    uv_index DOUBLE PRECISION,
    uv_exposure_minutes INTEGER,

    -- Environmental
    ambient_temperature_celsius DOUBLE PRECISION,
    humidity_percent DOUBLE PRECISION,
    air_pressure_hpa DOUBLE PRECISION,
    altitude_meters DOUBLE PRECISION,

    -- Time in Daylight
    time_in_daylight_minutes INTEGER,

    -- Metadata
    location_latitude DOUBLE PRECISION,
    location_longitude DOUBLE PRECISION,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at)
);

-- 6. Reproductive Health Metrics
CREATE TABLE IF NOT EXISTS reproductive_health_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Menstrual Cycle
    menstrual_flow VARCHAR(20), -- none, light, medium, heavy
    cervical_mucus_quality VARCHAR(20), -- dry, sticky, creamy, watery, eggwhite
    ovulation_test_result VARCHAR(20), -- negative, positive, indeterminate
    pregnancy_test_result VARCHAR(20), -- negative, positive, indeterminate
    progesterone_ng_per_ml DOUBLE PRECISION,

    -- Sexual Activity
    sexual_activity_occurred BOOLEAN,
    contraceptive_used BOOLEAN,

    -- Metadata
    cycle_day INTEGER,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at)
);

-- 7. Mobility & Balance Metrics
CREATE TABLE IF NOT EXISTS mobility_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Walking & Movement
    walking_speed_m_per_s DOUBLE PRECISION,
    walking_step_length_cm DOUBLE PRECISION,
    walking_asymmetry_percent DOUBLE PRECISION,
    walking_double_support_percent DOUBLE PRECISION,
    walking_steadiness_percent DOUBLE PRECISION,

    -- Mobility
    six_minute_walk_distance_meters DOUBLE PRECISION,
    stair_ascent_speed_m_per_s DOUBLE PRECISION,
    stair_descent_speed_m_per_s DOUBLE PRECISION,

    -- Balance & Coordination
    stand_time_seconds DOUBLE PRECISION,
    balance_score DOUBLE PRECISION,

    -- Metadata
    test_conditions VARCHAR(255),
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at, test_conditions)
);

-- 8. Symptoms & Health Events
CREATE TABLE IF NOT EXISTS health_symptoms (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Symptom Details
    symptom_type VARCHAR(100), -- headache, nausea, fatigue, pain, etc.
    severity INTEGER CHECK (severity >= 1 AND severity <= 10),
    body_location VARCHAR(100),
    duration_minutes INTEGER,

    -- Additional Context
    associated_activity VARCHAR(255),
    notes TEXT,

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- 9. Medications & Supplements
CREATE TABLE IF NOT EXISTS medications (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Medication Details
    medication_name VARCHAR(255) NOT NULL,
    dosage_amount DOUBLE PRECISION,
    dosage_unit VARCHAR(50), -- mg, ml, tablets, etc.
    frequency VARCHAR(100), -- once daily, twice daily, as needed

    -- Purpose & Notes
    purpose VARCHAR(255),
    prescriber VARCHAR(255),
    notes TEXT,

    -- Metadata
    start_date DATE,
    end_date DATE,
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, medication_name, recorded_at)
);

-- 10. Lab Results
CREATE TABLE IF NOT EXISTS lab_results (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Test Details
    test_name VARCHAR(255) NOT NULL,
    result_value DOUBLE PRECISION,
    result_unit VARCHAR(50),
    result_text VARCHAR(500), -- for non-numeric results

    -- Reference Range
    reference_range_low DOUBLE PRECISION,
    reference_range_high DOUBLE PRECISION,
    is_abnormal BOOLEAN,

    -- Provider Info
    lab_name VARCHAR(255),
    provider_name VARCHAR(255),

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, test_name, recorded_at)
);

-- Create indexes for performance
CREATE INDEX idx_blood_glucose_user_recorded ON blood_glucose_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_blood_glucose_user_source ON blood_glucose_metrics(user_id, glucose_source, recorded_at DESC);
CREATE INDEX idx_nutrition_user_recorded ON nutrition_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_micronutrients_user_recorded ON micronutrients(user_id, recorded_at DESC);
CREATE INDEX idx_body_metrics_user_recorded ON body_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_respiratory_user_recorded ON respiratory_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_environmental_user_recorded ON environmental_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_reproductive_user_recorded ON reproductive_health_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_mobility_user_recorded ON mobility_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_symptoms_user_recorded ON health_symptoms(user_id, recorded_at DESC);
CREATE INDEX idx_medications_user_recorded ON medications(user_id, recorded_at DESC);
CREATE INDEX idx_lab_results_user_recorded ON lab_results(user_id, recorded_at DESC);

-- 11. Mindfulness & Mental Health Metrics
CREATE TABLE IF NOT EXISTS mindfulness_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- Mindfulness Session Data
    session_duration_minutes INTEGER,
    meditation_type VARCHAR(50), -- guided, unguided, breathing, body_scan, walking
    session_quality_rating INTEGER CHECK (session_quality_rating >= 1 AND session_quality_rating <= 5),

    -- Mindful Minute Data
    mindful_minutes_today INTEGER,
    mindful_minutes_week INTEGER,

    -- Breathing & Focus
    breathing_rate_breaths_per_min DOUBLE PRECISION,
    heart_rate_variability_during_session DOUBLE PRECISION,
    focus_rating INTEGER CHECK (focus_rating >= 1 AND focus_rating <= 10),

    -- Session Context
    guided_session_instructor VARCHAR(255),
    meditation_app VARCHAR(100), -- calm, headspace, insight_timer, apple_mindfulness
    background_sounds VARCHAR(100), -- nature, rain, silence, music
    location_type VARCHAR(50), -- home, office, outdoors, studio

    -- Notes
    session_notes TEXT,

    -- Metadata
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at, meditation_type)
);

-- 12. Mental Health & State of Mind Metrics
CREATE TABLE IF NOT EXISTS mental_health_metrics (
    id UUID DEFAULT uuid_generate_v4() PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recorded_at TIMESTAMPTZ NOT NULL,

    -- iOS 17+ State of Mind Data
    state_of_mind_valence DOUBLE PRECISION CHECK (state_of_mind_valence >= -1.0 AND state_of_mind_valence <= 1.0), -- very unpleasant(-1) to very pleasant(1)
    state_of_mind_labels TEXT[], -- array of mood descriptors
    reflection_prompt TEXT,

    -- General Mood & Mental State
    mood_rating INTEGER CHECK (mood_rating >= 1 AND mood_rating <= 10),
    anxiety_level INTEGER CHECK (anxiety_level >= 1 AND anxiety_level <= 10),
    stress_level INTEGER CHECK (stress_level >= 1 AND stress_level <= 10),
    energy_level INTEGER CHECK (energy_level >= 1 AND energy_level <= 10),

    -- PHQ-9 Style Depression Screening (if applicable)
    depression_screening_score INTEGER CHECK (depression_screening_score >= 0 AND depression_screening_score <= 27),

    -- GAD-7 Style Anxiety Screening (if applicable)
    anxiety_screening_score INTEGER CHECK (anxiety_screening_score >= 0 AND anxiety_screening_score <= 21),

    -- Sleep Quality Impact on Mental Health
    sleep_quality_impact INTEGER CHECK (sleep_quality_impact >= 1 AND sleep_quality_impact <= 5),

    -- Context
    trigger_event VARCHAR(255), -- work_stress, relationship, health, financial
    coping_strategy VARCHAR(255), -- exercise, meditation, social_support, therapy
    medication_taken BOOLEAN,
    therapy_session_today BOOLEAN,

    -- Privacy Protected Notes (encrypted)
    private_notes_encrypted TEXT,
    notes_encryption_key_id UUID,

    -- Metadata
    data_sensitivity_level VARCHAR(20) DEFAULT 'high', -- high, medical, therapeutic
    source_device VARCHAR(255),
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,

    UNIQUE(user_id, recorded_at)
);

-- Create indexes for mindfulness and mental health
CREATE INDEX idx_mindfulness_user_recorded ON mindfulness_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_mental_health_user_recorded ON mental_health_metrics(user_id, recorded_at DESC);
CREATE INDEX idx_mindfulness_meditation_type ON mindfulness_metrics(meditation_type, recorded_at DESC);
CREATE INDEX idx_mental_health_mood_trends ON mental_health_metrics(user_id, mood_rating, recorded_at DESC);

-- Privacy and Security Indexes for Mental Health Data
CREATE INDEX idx_mental_health_sensitivity ON mental_health_metrics(data_sensitivity_level, user_id);

-- Add partitioning support for high-volume tables (optional)
-- This can be added later based on data volume