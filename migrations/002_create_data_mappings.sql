-- Create data_mappings table for HealthKit identifier mappings
-- This table stores the mapping between iOS HealthKit identifiers and our internal representation

CREATE TABLE IF NOT EXISTS data_mappings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    healthkit_identifier VARCHAR(255) UNIQUE NOT NULL,
    description TEXT NOT NULL,
    support_level VARCHAR(50) NOT NULL CHECK (support_level IN ('fully_supported', 'partial', 'planned', 'not_supported')),
    category VARCHAR(100) NOT NULL,
    notes TEXT,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP
);

-- Create index for faster lookups
CREATE INDEX IF NOT EXISTS idx_data_mappings_healthkit_id ON data_mappings(healthkit_identifier);
CREATE INDEX IF NOT EXISTS idx_data_mappings_category ON data_mappings(category);
CREATE INDEX IF NOT EXISTS idx_data_mappings_active ON data_mappings(is_active) WHERE is_active = true;

-- Insert core mappings as initial data
INSERT INTO data_mappings (healthkit_identifier, description, support_level, category, notes) VALUES
    ('HKQuantityTypeIdentifierStepCount', 'Step count', 'fully_supported', 'ACTIVITY', 'Daily step tracking'),
    ('HKQuantityTypeIdentifierHeartRate', 'Heart rate', 'fully_supported', 'HEART', 'Heart rate in beats per minute'),
    ('HKQuantityTypeIdentifierActiveEnergyBurned', 'Active calories', 'fully_supported', 'ENERGY', 'Active energy burned in kcal'),
    ('HKQuantityTypeIdentifierBodyMass', 'Body weight', 'fully_supported', 'BODY', 'Body weight measurements'),
    ('HKCategoryTypeIdentifierSleepAnalysis', 'Sleep stages', 'fully_supported', 'SLEEP', 'Sleep stage analysis'),
    ('HKQuantityTypeIdentifierBloodPressureSystolic', 'Systolic blood pressure', 'fully_supported', 'VITALS', 'Systolic BP in mmHg'),
    ('HKQuantityTypeIdentifierBloodPressureDiastolic', 'Diastolic blood pressure', 'fully_supported', 'VITALS', 'Diastolic BP in mmHg'),
    ('HKQuantityTypeIdentifierBodyTemperature', 'Body temperature', 'fully_supported', 'VITALS', 'Body temperature measurements'),
    ('HKQuantityTypeIdentifierOxygenSaturation', 'Blood oxygen', 'fully_supported', 'VITALS', 'SpO2 percentage'),
    ('HKQuantityTypeIdentifierRespiratoryRate', 'Respiratory rate', 'fully_supported', 'RESPIRATORY', 'Breaths per minute'),
    ('HKQuantityTypeIdentifierRestingHeartRate', 'Resting heart rate', 'fully_supported', 'HEART', 'Resting heart rate BPM'),
    ('HKQuantityTypeIdentifierHeartRateVariabilitySDNN', 'HRV (SDNN)', 'fully_supported', 'HEART', 'Heart rate variability'),
    ('HKQuantityTypeIdentifierBloodGlucose', 'Blood glucose', 'fully_supported', 'METABOLIC', 'Blood glucose in mg/dL'),
    ('HKQuantityTypeIdentifierInsulinDelivery', 'Insulin delivery', 'fully_supported', 'METABOLIC', 'Insulin units delivered'),
    ('HKQuantityTypeIdentifierDistanceWalkingRunning', 'Walking/running distance', 'fully_supported', 'ACTIVITY', 'Distance in meters'),
    ('HKQuantityTypeIdentifierDistanceCycling', 'Cycling distance', 'fully_supported', 'ACTIVITY', 'Cycling distance in meters'),
    ('HKQuantityTypeIdentifierDistanceSwimming', 'Swimming distance', 'fully_supported', 'ACTIVITY', 'Swimming distance in meters'),
    ('HKQuantityTypeIdentifierFlightsClimbed', 'Flights climbed', 'fully_supported', 'ACTIVITY', 'Number of flights climbed'),
    ('HKQuantityTypeIdentifierEnvironmentalAudioExposure', 'Audio exposure', 'fully_supported', 'ENVIRONMENTAL', 'Environmental noise exposure'),
    ('HKQuantityTypeIdentifierUVExposure', 'UV exposure', 'fully_supported', 'ENVIRONMENTAL', 'UV index exposure'),
    ('HKCategoryTypeIdentifierMindfulSession', 'Mindfulness session', 'fully_supported', 'MENTAL_HEALTH', 'Meditation/mindfulness sessions'),
    ('HKCategoryTypeIdentifierHandwashingEvent', 'Handwashing', 'fully_supported', 'HYGIENE', 'Hand hygiene events'),
    ('HKCategoryTypeIdentifierToothbrushingEvent', 'Toothbrushing', 'fully_supported', 'HYGIENE', 'Oral hygiene events')
ON CONFLICT (healthkit_identifier) DO NOTHING;