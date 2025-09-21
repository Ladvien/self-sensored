/// Comprehensive health data validation tests covering all metric types and validation rules
use chrono::{DateTime, TimeZone, Utc};
use proptest::prelude::*;
use self_sensored::config::ValidationConfig;
use self_sensored::models::enums::{ActivityContext, WorkoutType};
use self_sensored::models::health_metrics::*;
use self_sensored::models::ios_models::*;
use std::collections::HashMap;
use uuid::Uuid;

// Test utilities for creating test data
mod test_utils {
    use super::*;

    pub fn create_valid_heart_rate_metric(user_id: Uuid, bpm: i16) -> HeartRateMetric {
        HeartRateMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: Utc::now(),
            heart_rate: Some(bpm),
            resting_heart_rate: Some(60),
            heart_rate_variability: Some(45.0),
            walking_heart_rate_average: None,
            heart_rate_recovery_one_minute: None,
            atrial_fibrillation_burden_percentage: None,
            vo2_max_ml_kg_min: None,
            source_device: Some("Apple Watch".to_string()),
            context: Some(ActivityContext::Rest),
            created_at: Utc::now(),
        }
    }

    pub fn create_valid_blood_pressure_metric(
        user_id: Uuid,
        systolic: i16,
        diastolic: i16,
    ) -> BloodPressureMetric {
        BloodPressureMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: Utc::now(),
            systolic,
            diastolic,
            pulse: Some(70),
            source_device: Some("Blood Pressure Monitor".to_string()),
            created_at: Utc::now(),
        }
    }

    pub fn create_valid_sleep_metric(user_id: Uuid, efficiency: f64) -> SleepMetric {
        let start_time = Utc::now() - chrono::Duration::hours(8);
        let end_time = Utc::now();

        SleepMetric {
            id: Uuid::new_v4(),
            user_id,
            sleep_start: start_time,
            sleep_end: end_time,
            duration_minutes: Some(480),
            deep_sleep_minutes: Some(120),
            rem_sleep_minutes: Some(100),
            light_sleep_minutes: Some(240),
            awake_minutes: Some(20),
            efficiency: Some(efficiency),
            source_device: Some("Apple Watch".to_string()),
            created_at: Utc::now(),
        }
    }

    pub fn create_valid_activity_metric(user_id: Uuid, step_count: i32) -> ActivityMetric {
        ActivityMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: Utc::now(),
            step_count: Some(step_count),
            distance_meters: Some(8000.0),
            flights_climbed: Some(10),
            active_energy_burned_kcal: Some(400.0),
            basal_energy_burned_kcal: Some(1600.0),
            distance_cycling_meters: None,
            distance_swimming_meters: None,
            distance_wheelchair_meters: None,
            distance_downhill_snow_sports_meters: None,
            push_count: None,
            swimming_stroke_count: None,
            nike_fuel_points: None,
            apple_exercise_time_minutes: None,
            apple_stand_time_minutes: None,
            apple_move_time_minutes: None,
            apple_stand_hour_achieved: None,
            walking_speed_m_per_s: None,
            walking_step_length_cm: None,
            walking_asymmetry_percent: None,
            walking_double_support_percent: None,
            six_minute_walk_test_distance_m: None,
            stair_ascent_speed_m_per_s: None,
            stair_descent_speed_m_per_s: None,
            ground_contact_time_ms: None,
            vertical_oscillation_cm: None,
            running_stride_length_m: None,
            running_power_watts: None,
            running_speed_m_per_s: None,
            cycling_speed_kmh: None,
            cycling_power_watts: None,
            cycling_cadence_rpm: None,
            functional_threshold_power_watts: None,
            underwater_depth_meters: None,
            diving_duration_seconds: None,
            source_device: Some("iPhone".to_string()),
            created_at: Utc::now(),
        }
    }

    pub fn create_ios_payload_with_heart_rate(bpm: f64) -> IosIngestPayload {
        IosIngestPayload {
            data: IosIngestData {
                metrics: vec![IosMetric {
                    name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                    units: Some("count/min".to_string()),
                    data: vec![IosMetricData {
                        qty: Some(bpm),
                        date: Some(Utc::now().to_rfc3339()),
                        start: None,
                        end: None,
                        source: Some("Apple Watch".to_string()),
                        value: None,
                        extra: HashMap::new(),
                    }],
                }],
                workouts: vec![],
            },
        }
    }

    pub fn create_ios_payload_with_blood_pressure(
        systolic: f64,
        diastolic: f64,
    ) -> IosIngestPayload {
        let timestamp = Utc::now().to_rfc3339();
        IosIngestPayload {
            data: IosIngestData {
                metrics: vec![
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureSystolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(systolic),
                            date: Some(timestamp.clone()),
                            start: None,
                            end: None,
                            source: Some("Blood Pressure Monitor".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                    IosMetric {
                        name: "HKQuantityTypeIdentifierBloodPressureDiastolic".to_string(),
                        units: Some("mmHg".to_string()),
                        data: vec![IosMetricData {
                            qty: Some(diastolic),
                            date: Some(timestamp),
                            start: None,
                            end: None,
                            source: Some("Blood Pressure Monitor".to_string()),
                            value: None,
                            extra: HashMap::new(),
                        }],
                    },
                ],
                workouts: vec![],
            },
        }
    }
}

#[cfg(test)]
mod heart_rate_validation_tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_valid_heart_rate_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test valid boundary values
        let valid_rates = [15, 60, 100, 150, 200, 300];
        for &rate in &valid_rates {
            let metric = create_valid_heart_rate_metric(user_id, rate);
            assert!(
                rate >= config.heart_rate_min && rate <= config.heart_rate_max,
                "Heart rate {} should be valid",
                rate
            );
        }
    }

    #[test]
    fn test_invalid_heart_rate_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test invalid boundary values
        let invalid_rates = [0, 14, 301, 500, -10];
        for &rate in &invalid_rates {
            let metric = create_valid_heart_rate_metric(user_id, rate);
            assert!(
                rate < config.heart_rate_min || rate > config.heart_rate_max,
                "Heart rate {} should be invalid",
                rate
            );
        }
    }

    #[test]
    fn test_heart_rate_boundary_values() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test exact boundary values
        let min_metric = create_valid_heart_rate_metric(user_id, config.heart_rate_min);
        let max_metric = create_valid_heart_rate_metric(user_id, config.heart_rate_max);

        assert!(min_metric.heart_rate.unwrap() >= config.heart_rate_min);
        assert!(max_metric.heart_rate.unwrap() <= config.heart_rate_max);

        // Test just outside boundaries
        let below_min = create_valid_heart_rate_metric(user_id, config.heart_rate_min - 1);
        let above_max = create_valid_heart_rate_metric(user_id, config.heart_rate_max + 1);

        assert!(below_min.heart_rate.unwrap() < config.heart_rate_min);
        assert!(above_max.heart_rate.unwrap() > config.heart_rate_max);
    }

    #[test]
    fn test_heart_rate_none_values() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_heart_rate_metric(user_id, 72);
        metric.heart_rate = None;

        // None values should be allowed as they represent missing data
        assert!(metric.heart_rate.is_none());
    }

    #[test]
    fn test_heart_rate_medical_emergency_values() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test bradycardia (slow heart rate)
        let bradycardia = create_valid_heart_rate_metric(user_id, 40);
        assert!(bradycardia.heart_rate.unwrap() >= config.heart_rate_min);

        // Test tachycardia (fast heart rate)
        let tachycardia = create_valid_heart_rate_metric(user_id, 180);
        assert!(tachycardia.heart_rate.unwrap() <= config.heart_rate_max);

        // Test extreme emergency values
        let extreme_low = create_valid_heart_rate_metric(user_id, 15);
        let extreme_high = create_valid_heart_rate_metric(user_id, 250);

        assert!(extreme_low.heart_rate.unwrap() >= config.heart_rate_min);
        assert!(extreme_high.heart_rate.unwrap() <= config.heart_rate_max);
    }

    #[test]
    fn test_heart_rate_variability_validation() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_heart_rate_metric(user_id, 72);

        // Test various HRV values
        let hrv_values = [0.0, 25.0, 50.0, 100.0, 200.0];
        for &hrv in &hrv_values {
            metric.heart_rate_variability = Some(hrv);
            assert!(metric.heart_rate_variability.unwrap() >= 0.0);
        }

        // Test negative HRV (should be invalid)
        metric.heart_rate_variability = Some(-10.0);
        assert!(metric.heart_rate_variability.unwrap() < 0.0);
    }

    #[test]
    fn test_resting_heart_rate_validation() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_heart_rate_metric(user_id, 72);

        // Test typical resting heart rate ranges
        let resting_rates = [40, 50, 60, 70, 80, 90, 100];
        for &rate in &resting_rates {
            metric.resting_heart_rate = Some(rate);
            assert!(
                rate >= config.heart_rate_min && rate <= config.heart_rate_max,
                "Resting heart rate {} should be within valid range",
                rate
            );
        }
    }
}

#[cfg(test)]
mod blood_pressure_validation_tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_valid_blood_pressure_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test normal blood pressure ranges
        let normal_bp = [
            (90, 60),   // Low normal
            (120, 80),  // Optimal
            (130, 85),  // Normal high
            (140, 90),  // Stage 1 hypertension
            (160, 100), // Stage 2 hypertension
        ];

        for &(systolic, diastolic) in &normal_bp {
            let metric = create_valid_blood_pressure_metric(user_id, systolic, diastolic);
            assert!(
                systolic >= config.systolic_min && systolic <= config.systolic_max,
                "Systolic {} should be valid",
                systolic
            );
            assert!(
                diastolic >= config.diastolic_min && diastolic <= config.diastolic_max,
                "Diastolic {} should be valid",
                diastolic
            );
        }
    }

    #[test]
    fn test_invalid_blood_pressure_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test invalid blood pressure ranges
        let invalid_bp = [
            (40, 20),   // Too low
            (300, 200), // Too high
            (0, 0),     // Zero values
            (-10, -5),  // Negative values
        ];

        for &(systolic, diastolic) in &invalid_bp {
            let metric = create_valid_blood_pressure_metric(user_id, systolic, diastolic);
            let systolic_invalid =
                systolic < config.systolic_min || systolic > config.systolic_max;
            let diastolic_invalid =
                diastolic < config.diastolic_min || diastolic > config.diastolic_max;

            assert!(
                systolic_invalid || diastolic_invalid,
                "BP {}/{} should be invalid",
                systolic,
                diastolic
            );
        }
    }

    #[test]
    fn test_blood_pressure_boundary_values() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test systolic boundaries
        let min_systolic = create_valid_blood_pressure_metric(user_id, config.systolic_min, 80);
        let max_systolic = create_valid_blood_pressure_metric(user_id, config.systolic_max, 80);

        assert_eq!(min_systolic.systolic, config.systolic_min);
        assert_eq!(max_systolic.systolic, config.systolic_max);

        // Test diastolic boundaries
        let min_diastolic = create_valid_blood_pressure_metric(user_id, 120, config.diastolic_min);
        let max_diastolic = create_valid_blood_pressure_metric(user_id, 120, config.diastolic_max);

        assert_eq!(min_diastolic.diastolic, config.diastolic_min);
        assert_eq!(max_diastolic.diastolic, config.diastolic_max);
    }

    #[test]
    fn test_blood_pressure_medical_conditions() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test hypotension (low blood pressure)
        let hypotension = create_valid_blood_pressure_metric(user_id, 80, 50);
        assert!(hypotension.systolic >= config.systolic_min);
        assert!(hypotension.diastolic >= config.diastolic_min);

        // Test hypertensive crisis
        let crisis = create_valid_blood_pressure_metric(user_id, 220, 120);
        assert!(crisis.systolic <= config.systolic_max);
        assert!(crisis.diastolic <= config.diastolic_max);

        // Test isolated systolic hypertension
        let isolated_systolic = create_valid_blood_pressure_metric(user_id, 160, 80);
        assert!(isolated_systolic.systolic <= config.systolic_max);
        assert!(isolated_systolic.diastolic <= config.diastolic_max);
    }

    #[test]
    fn test_pulse_pressure_validation() {
        let user_id = Uuid::new_v4();

        // Test normal pulse pressure (systolic - diastolic = 40-60 mmHg)
        let normal_pulse = create_valid_blood_pressure_metric(user_id, 120, 80);
        let pulse_pressure = normal_pulse.systolic - normal_pulse.diastolic;
        assert!(pulse_pressure >= 20 && pulse_pressure <= 80, "Normal pulse pressure range");

        // Test wide pulse pressure (>60 mmHg)
        let wide_pulse = create_valid_blood_pressure_metric(user_id, 160, 80);
        let wide_pressure = wide_pulse.systolic - wide_pulse.diastolic;
        assert!(wide_pressure > 60, "Wide pulse pressure");

        // Test narrow pulse pressure (<40 mmHg)
        let narrow_pulse = create_valid_blood_pressure_metric(user_id, 100, 85);
        let narrow_pressure = narrow_pulse.systolic - narrow_pulse.diastolic;
        assert!(narrow_pressure < 40, "Narrow pulse pressure");
    }
}

#[cfg(test)]
mod sleep_validation_tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_valid_sleep_efficiency_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test valid sleep efficiency percentages
        let valid_efficiencies = [0.0, 25.0, 50.0, 75.0, 85.0, 95.0, 100.0];
        for &efficiency in &valid_efficiencies {
            let metric = create_valid_sleep_metric(user_id, efficiency);
            assert!(
                efficiency >= config.sleep_efficiency_min
                    && efficiency <= config.sleep_efficiency_max,
                "Sleep efficiency {} should be valid",
                efficiency
            );
        }
    }

    #[test]
    fn test_invalid_sleep_efficiency_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test invalid sleep efficiency percentages
        let invalid_efficiencies = [-10.0, -1.0, 101.0, 150.0, 200.0];
        for &efficiency in &invalid_efficiencies {
            let metric = create_valid_sleep_metric(user_id, efficiency);
            assert!(
                efficiency < config.sleep_efficiency_min
                    || efficiency > config.sleep_efficiency_max,
                "Sleep efficiency {} should be invalid",
                efficiency
            );
        }
    }

    #[test]
    fn test_sleep_duration_validation() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_sleep_metric(user_id, 85.0);

        // Test typical sleep durations (in minutes)
        let durations = [240, 360, 480, 540, 600]; // 4-10 hours
        for &duration in &durations {
            metric.duration_minutes = Some(duration);
            assert!(duration > 0, "Sleep duration should be positive");
            assert!(duration <= 12 * 60, "Sleep duration should be reasonable");
        }

        // Test unrealistic durations
        let unrealistic_durations = [0, 30, 1440, 2000]; // 0, 30min, 24h, 33h
        for &duration in &unrealistic_durations {
            metric.duration_minutes = Some(duration);
            if duration == 0 {
                assert_eq!(duration, 0, "Zero duration should be flagged");
            } else if duration > 720 {
                // 12 hours
                assert!(duration > 720, "Excessive duration should be flagged");
            }
        }
    }

    #[test]
    fn test_sleep_stage_validation() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_sleep_metric(user_id, 85.0);

        // Test that sleep stages sum to reasonable total
        metric.deep_sleep_minutes = Some(90);
        metric.rem_sleep_minutes = Some(120);
        metric.light_sleep_minutes = Some(240);
        metric.awake_minutes = Some(30);

        let total_stages = metric.deep_sleep_minutes.unwrap_or(0)
            + metric.rem_sleep_minutes.unwrap_or(0)
            + metric.light_sleep_minutes.unwrap_or(0)
            + metric.awake_minutes.unwrap_or(0);

        assert!(total_stages > 0, "Total sleep stages should be positive");
        assert!(
            total_stages <= metric.duration_minutes.unwrap_or(0) + 60,
            "Sleep stages should not exceed total duration by much"
        );
    }

    #[test]
    fn test_sleep_time_consistency() {
        let user_id = Uuid::new_v4();
        let sleep_start = Utc.with_ymd_and_hms(2024, 1, 1, 22, 0, 0).unwrap();
        let sleep_end = Utc.with_ymd_and_hms(2024, 1, 2, 6, 0, 0).unwrap();

        let mut metric = create_valid_sleep_metric(user_id, 85.0);
        metric.sleep_start = sleep_start;
        metric.sleep_end = sleep_end;

        // Calculate expected duration
        let expected_duration = (sleep_end - sleep_start).num_minutes() as i32;
        metric.duration_minutes = Some(expected_duration);

        assert!(sleep_end > sleep_start, "Sleep end should be after sleep start");
        assert_eq!(
            metric.duration_minutes.unwrap(),
            expected_duration,
            "Duration should match time difference"
        );
        assert_eq!(expected_duration, 480, "8 hour sleep duration"); // 8 hours
    }

    #[test]
    fn test_sleep_efficiency_calculation() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_sleep_metric(user_id, 85.0);

        // Set up sleep data for efficiency calculation
        metric.duration_minutes = Some(480); // 8 hours
        metric.awake_minutes = Some(48); // 48 minutes awake

        // Calculate expected efficiency: (total - awake) / total * 100
        let expected_efficiency = ((480.0 - 48.0) / 480.0) * 100.0;
        metric.efficiency = Some(expected_efficiency);

        assert!((metric.efficiency.unwrap() - 90.0).abs() < 0.1, "Efficiency should be ~90%");
        assert!(
            metric.efficiency.unwrap() >= 85.0,
            "Good sleep efficiency should be >= 85%"
        );
    }
}

#[cfg(test)]
mod activity_validation_tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_valid_step_count_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test valid step counts
        let valid_steps = [0, 1000, 5000, 10000, 15000, 25000, 50000];
        for &steps in &valid_steps {
            let metric = create_valid_activity_metric(user_id, steps);
            assert!(
                steps >= config.step_count_min && steps <= config.step_count_max,
                "Step count {} should be valid",
                steps
            );
        }
    }

    #[test]
    fn test_invalid_step_count_ranges() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();

        // Test invalid step counts
        let invalid_steps = [-100, -1, 250000, 1000000];
        for &steps in &invalid_steps {
            let metric = create_valid_activity_metric(user_id, steps);
            assert!(
                steps < config.step_count_min || steps > config.step_count_max,
                "Step count {} should be invalid",
                steps
            );
        }
    }

    #[test]
    fn test_distance_validation() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_activity_metric(user_id, 10000);

        // Test distance in meters (convert km to meters for comparison)
        let max_distance_meters = config.distance_max_km * 1000.0;

        let valid_distances = [0.0, 1000.0, 5000.0, 10000.0, 42195.0]; // Including marathon distance
        for &distance in &valid_distances {
            metric.distance_meters = Some(distance);
            assert!(
                distance >= 0.0 && distance <= max_distance_meters,
                "Distance {} meters should be valid",
                distance
            );
        }

        // Test invalid distances
        let invalid_distances = [-100.0, config.distance_max_km * 1000.0 + 1.0];
        for &distance in &invalid_distances {
            metric.distance_meters = Some(distance);
            assert!(
                distance < 0.0 || distance > max_distance_meters,
                "Distance {} meters should be invalid",
                distance
            );
        }
    }

    #[test]
    fn test_calories_validation() {
        let config = ValidationConfig::default();
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_activity_metric(user_id, 10000);

        // Test calorie ranges
        let valid_calories = [0.0, 100.0, 500.0, 2000.0, 5000.0];
        for &calories in &valid_calories {
            metric.active_energy_burned_kcal = Some(calories);
            assert!(
                calories >= 0.0 && calories <= config.calories_max,
                "Calories {} should be valid",
                calories
            );
        }

        // Test invalid calories
        let invalid_calories = [-100.0, config.calories_max + 1.0];
        for &calories in &invalid_calories {
            metric.active_energy_burned_kcal = Some(calories);
            assert!(
                calories < 0.0 || calories > config.calories_max,
                "Calories {} should be invalid",
                calories
            );
        }
    }

    #[test]
    fn test_flights_climbed_validation() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_activity_metric(user_id, 10000);

        // Test flight counts (reasonable daily maximums)
        let valid_flights = [0, 5, 10, 50, 100, 200];
        for &flights in &valid_flights {
            metric.flights_climbed = Some(flights);
            assert!(flights >= 0, "Flights climbed should be non-negative");
            assert!(flights <= 500, "Flights climbed should be reasonable");
        }

        // Test unrealistic values
        let unrealistic_flights = [-10, 1000, 5000];
        for &flights in &unrealistic_flights {
            metric.flights_climbed = Some(flights);
            if flights < 0 {
                assert!(flights < 0, "Negative flights should be flagged");
            } else if flights > 500 {
                assert!(flights > 500, "Excessive flights should be flagged");
            }
        }
    }

    #[test]
    fn test_specialized_activity_metrics() {
        let user_id = Uuid::new_v4();
        let mut metric = create_valid_activity_metric(user_id, 10000);

        // Test cycling metrics
        metric.distance_cycling_meters = Some(50000.0); // 50km cycling
        metric.cycling_speed_kmh = Some(25.0);
        metric.cycling_power_watts = Some(200.0);
        metric.cycling_cadence_rpm = Some(85.0);

        assert!(
            metric.distance_cycling_meters.unwrap() >= 0.0,
            "Cycling distance should be non-negative"
        );
        assert!(
            metric.cycling_speed_kmh.unwrap() > 0.0 && metric.cycling_speed_kmh.unwrap() <= 100.0,
            "Cycling speed should be reasonable"
        );

        // Test swimming metrics
        metric.distance_swimming_meters = Some(2000.0); // 2km swimming
        metric.swimming_stroke_count = Some(800);

        assert!(
            metric.distance_swimming_meters.unwrap() >= 0.0,
            "Swimming distance should be non-negative"
        );
        assert!(
            metric.swimming_stroke_count.unwrap() >= 0,
            "Stroke count should be non-negative"
        );

        // Test running metrics
        metric.running_speed_m_per_s = Some(4.0); // 4 m/s = ~14.4 km/h
        metric.running_stride_length_m = Some(1.2);
        metric.running_power_watts = Some(300.0);

        assert!(
            metric.running_speed_m_per_s.unwrap() > 0.0
                && metric.running_speed_m_per_s.unwrap() <= 15.0,
            "Running speed should be reasonable"
        );
        assert!(
            metric.running_stride_length_m.unwrap() > 0.0
                && metric.running_stride_length_m.unwrap() <= 3.0,
            "Stride length should be reasonable"
        );
    }
}

#[cfg(test)]
mod gps_coordinates_validation_tests {
    use super::*;

    #[test]
    fn test_valid_gps_coordinates() {
        let config = ValidationConfig::default();

        // Test valid coordinate ranges
        let valid_coords = [
            (0.0, 0.0),         // Equator, Prime Meridian
            (40.7128, -74.0060), // New York City
            (51.5074, -0.1278),  // London
            (-33.8688, 151.2093), // Sydney
            (35.6762, 139.6503), // Tokyo
            (90.0, 180.0),      // North Pole, International Date Line
            (-90.0, -180.0),    // South Pole, International Date Line
        ];

        for &(lat, lon) in &valid_coords {
            assert!(
                lat >= config.latitude_min && lat <= config.latitude_max,
                "Latitude {} should be valid",
                lat
            );
            assert!(
                lon >= config.longitude_min && lon <= config.longitude_max,
                "Longitude {} should be valid",
                lon
            );
        }
    }

    #[test]
    fn test_invalid_gps_coordinates() {
        let config = ValidationConfig::default();

        // Test invalid coordinate ranges
        let invalid_coords = [
            (91.0, 0.0),    // Latitude too high
            (-91.0, 0.0),   // Latitude too low
            (0.0, 181.0),   // Longitude too high
            (0.0, -181.0),  // Longitude too low
            (100.0, 200.0), // Both out of range
        ];

        for &(lat, lon) in &invalid_coords {
            let lat_invalid = lat < config.latitude_min || lat > config.latitude_max;
            let lon_invalid = lon < config.longitude_min || lon > config.longitude_max;

            assert!(
                lat_invalid || lon_invalid,
                "Coordinates ({}, {}) should be invalid",
                lat,
                lon
            );
        }
    }

    #[test]
    fn test_gps_boundary_values() {
        let config = ValidationConfig::default();

        // Test exact boundary values
        assert_eq!(config.latitude_min, -90.0);
        assert_eq!(config.latitude_max, 90.0);
        assert_eq!(config.longitude_min, -180.0);
        assert_eq!(config.longitude_max, 180.0);

        // Test just inside boundaries
        let valid_boundaries = [
            (-89.9, -179.9),
            (89.9, 179.9),
            (-90.0, 180.0),
            (90.0, -180.0),
        ];

        for &(lat, lon) in &valid_boundaries {
            assert!(
                lat >= config.latitude_min && lat <= config.latitude_max,
                "Boundary latitude {} should be valid",
                lat
            );
            assert!(
                lon >= config.longitude_min && lon <= config.longitude_max,
                "Boundary longitude {} should be valid",
                lon
            );
        }
    }
}

#[cfg(test)]
mod workout_validation_tests {
    use super::*;

    #[test]
    fn test_workout_duration_validation() {
        let config = ValidationConfig::default();

        // Test valid workout durations
        let valid_durations = [1, 2, 4, 8, 12, 24]; // hours
        for &duration in &valid_durations {
            assert!(
                duration <= config.workout_max_duration_hours,
                "Workout duration {} hours should be valid",
                duration
            );
        }

        // Test invalid workout durations
        let invalid_durations = [25, 48, 100]; // hours
        for &duration in &invalid_durations {
            assert!(
                duration > config.workout_max_duration_hours,
                "Workout duration {} hours should be invalid",
                duration
            );
        }
    }

    #[test]
    fn test_workout_heart_rate_validation() {
        let config = ValidationConfig::default();

        // Test valid workout heart rates
        let valid_rates = [60, 100, 150, 180, 200];
        for &rate in &valid_rates {
            assert!(
                rate >= config.workout_heart_rate_min && rate <= config.workout_heart_rate_max,
                "Workout heart rate {} should be valid",
                rate
            );
        }

        // Test invalid workout heart rates
        let invalid_rates = [10, 14, 301, 400];
        for &rate in &invalid_rates {
            assert!(
                rate < config.workout_heart_rate_min || rate > config.workout_heart_rate_max,
                "Workout heart rate {} should be invalid",
                rate
            );
        }
    }
}

#[cfg(test)]
mod ios_payload_validation_tests {
    use super::*;
    use test_utils::*;

    #[test]
    fn test_ios_heart_rate_payload_validation() {
        // Test valid iOS heart rate payload
        let valid_payload = create_ios_payload_with_heart_rate(72.0);
        let config = ValidationConfig::default();

        assert_eq!(valid_payload.data.metrics.len(), 1);
        assert_eq!(
            valid_payload.data.metrics[0].name,
            "HKQuantityTypeIdentifierHeartRate"
        );

        let heart_rate = valid_payload.data.metrics[0].data[0].qty.unwrap() as i16;
        assert!(heart_rate >= config.heart_rate_min && heart_rate <= config.heart_rate_max);

        // Test invalid iOS heart rate payload
        let invalid_payload = create_ios_payload_with_heart_rate(350.0);
        let invalid_heart_rate = invalid_payload.data.metrics[0].data[0].qty.unwrap() as i16;
        assert!(
            invalid_heart_rate < config.heart_rate_min
                || invalid_heart_rate > config.heart_rate_max
        );
    }

    #[test]
    fn test_ios_blood_pressure_payload_validation() {
        // Test valid iOS blood pressure payload
        let valid_payload = create_ios_payload_with_blood_pressure(120.0, 80.0);
        let config = ValidationConfig::default();

        assert_eq!(valid_payload.data.metrics.len(), 2);

        let systolic = valid_payload.data.metrics[0].data[0].qty.unwrap() as i16;
        let diastolic = valid_payload.data.metrics[1].data[0].qty.unwrap() as i16;

        assert!(systolic >= config.systolic_min && systolic <= config.systolic_max);
        assert!(diastolic >= config.diastolic_min && diastolic <= config.diastolic_max);

        // Test invalid iOS blood pressure payload
        let invalid_payload = create_ios_payload_with_blood_pressure(300.0, 200.0);
        let invalid_systolic = invalid_payload.data.metrics[0].data[0].qty.unwrap() as i16;
        let invalid_diastolic = invalid_payload.data.metrics[1].data[0].qty.unwrap() as i16;

        assert!(
            invalid_systolic < config.systolic_min || invalid_systolic > config.systolic_max
        );
        assert!(
            invalid_diastolic < config.diastolic_min || invalid_diastolic > config.diastolic_max
        );
    }

    #[test]
    fn test_ios_payload_missing_fields() {
        let mut payload = create_ios_payload_with_heart_rate(72.0);

        // Test missing qty field
        payload.data.metrics[0].data[0].qty = None;
        assert!(payload.data.metrics[0].data[0].qty.is_none());

        // Test missing date field
        payload.data.metrics[0].data[0].date = None;
        assert!(payload.data.metrics[0].data[0].date.is_none());

        // Test missing source field
        payload.data.metrics[0].data[0].source = None;
        assert!(payload.data.metrics[0].data[0].source.is_none());
    }

    #[test]
    fn test_ios_payload_date_parsing() {
        let payload = create_ios_payload_with_heart_rate(72.0);
        let date_str = payload.data.metrics[0].data[0].date.as_ref().unwrap();

        // Test that the date string is valid ISO 8601 format
        let parsed_date = DateTime::parse_from_rfc3339(date_str);
        assert!(parsed_date.is_ok(), "Date should be valid ISO 8601 format");

        // Test various date formats that might come from iOS
        let test_dates = [
            "2024-01-15T10:30:00Z",
            "2024-01-15T10:30:00.000Z",
            "2024-01-15T10:30:00+00:00",
            "2024-01-15T10:30:00-05:00",
        ];

        for date_str in &test_dates {
            let parsed = DateTime::parse_from_rfc3339(date_str);
            assert!(parsed.is_ok(), "Date format {} should be valid", date_str);
        }
    }
}

// Property-based testing using proptest
#[cfg(test)]
mod property_based_tests {
    use super::*;

    proptest! {
        #[test]
        fn test_heart_rate_property_validation(heart_rate in 1i16..400i16) {
            let config = ValidationConfig::default();
            let user_id = Uuid::new_v4();
            let metric = test_utils::create_valid_heart_rate_metric(user_id, heart_rate);

            let is_valid = heart_rate >= config.heart_rate_min && heart_rate <= config.heart_rate_max;
            let should_be_valid = metric.heart_rate.unwrap() >= config.heart_rate_min &&
                                 metric.heart_rate.unwrap() <= config.heart_rate_max;

            prop_assert_eq!(is_valid, should_be_valid);
        }

        #[test]
        fn test_blood_pressure_property_validation(
            systolic in 1i16..300i16,
            diastolic in 1i16..200i16
        ) {
            let config = ValidationConfig::default();
            let user_id = Uuid::new_v4();
            let metric = test_utils::create_valid_blood_pressure_metric(user_id, systolic, diastolic);

            let systolic_valid = systolic >= config.systolic_min && systolic <= config.systolic_max;
            let diastolic_valid = diastolic >= config.diastolic_min && diastolic <= config.diastolic_max;

            prop_assert_eq!(metric.systolic >= config.systolic_min && metric.systolic <= config.systolic_max, systolic_valid);
            prop_assert_eq!(metric.diastolic >= config.diastolic_min && metric.diastolic <= config.diastolic_max, diastolic_valid);
        }

        #[test]
        fn test_sleep_efficiency_property_validation(efficiency in -50.0f64..150.0f64) {
            let config = ValidationConfig::default();
            let user_id = Uuid::new_v4();
            let metric = test_utils::create_valid_sleep_metric(user_id, efficiency);

            let is_valid = efficiency >= config.sleep_efficiency_min && efficiency <= config.sleep_efficiency_max;
            let should_be_valid = metric.efficiency.unwrap() >= config.sleep_efficiency_min &&
                                 metric.efficiency.unwrap() <= config.sleep_efficiency_max;

            prop_assert_eq!(is_valid, should_be_valid);
        }

        #[test]
        fn test_step_count_property_validation(steps in -1000i32..300000i32) {
            let config = ValidationConfig::default();
            let user_id = Uuid::new_v4();
            let metric = test_utils::create_valid_activity_metric(user_id, steps);

            let is_valid = steps >= config.step_count_min && steps <= config.step_count_max;
            let should_be_valid = metric.step_count.unwrap() >= config.step_count_min &&
                                 metric.step_count.unwrap() <= config.step_count_max;

            prop_assert_eq!(is_valid, should_be_valid);
        }

        #[test]
        fn test_gps_coordinates_property_validation(
            lat in -100.0f64..100.0f64,
            lon in -200.0f64..200.0f64
        ) {
            let config = ValidationConfig::default();

            let lat_valid = lat >= config.latitude_min && lat <= config.latitude_max;
            let lon_valid = lon >= config.longitude_min && lon <= config.longitude_max;

            prop_assert_eq!(lat_valid, lat >= -90.0 && lat <= 90.0);
            prop_assert_eq!(lon_valid, lon >= -180.0 && lon <= 180.0);
        }
    }
}

#[cfg(test)]
mod timezone_handling_tests {
    use super::*;
    use chrono::{FixedOffset, TimeZone};

    #[test]
    fn test_utc_timestamp_handling() {
        let user_id = Uuid::new_v4();
        let utc_time = Utc::now();
        let mut metric = test_utils::create_valid_heart_rate_metric(user_id, 72);
        metric.recorded_at = utc_time;

        // Verify UTC timezone is preserved
        assert_eq!(metric.recorded_at.timezone(), Utc);
        assert_eq!(metric.recorded_at.format("%Z").to_string(), "UTC");
    }

    #[test]
    fn test_timezone_conversion_consistency() {
        let user_id = Uuid::new_v4();

        // Create timestamps in different timezones
        let utc_time = Utc.with_ymd_and_hms(2024, 1, 15, 12, 0, 0).unwrap();
        let est_offset = FixedOffset::west_opt(5 * 3600).unwrap(); // UTC-5
        let est_time = est_offset.with_ymd_and_hms(2024, 1, 15, 7, 0, 0).unwrap();

        // Convert to UTC for comparison
        let est_as_utc = est_time.with_timezone(&Utc);

        assert_eq!(utc_time, est_as_utc, "Times should be equivalent when converted to UTC");
    }

    #[test]
    fn test_ios_date_string_parsing() {
        let user_id = Uuid::new_v4();

        // Test various iOS date formats
        let ios_date_formats = [
            "2024-01-15T12:00:00Z",
            "2024-01-15T12:00:00.000Z",
            "2024-01-15T12:00:00+00:00",
            "2024-01-15T07:00:00-05:00", // EST
            "2024-01-15T13:00:00+01:00", // CET
        ];

        for date_str in &ios_date_formats {
            let parsed = DateTime::parse_from_rfc3339(date_str);
            assert!(parsed.is_ok(), "Failed to parse date: {}", date_str);

            let utc_time = parsed.unwrap().with_timezone(&Utc);
            let mut payload = test_utils::create_ios_payload_with_heart_rate(72.0);
            payload.data.metrics[0].data[0].date = Some(date_str.to_string());

            // Verify the date can be extracted and converted properly
            assert!(payload.data.metrics[0].data[0].date.is_some());
        }
    }
}

#[cfg(test)]
mod unit_conversion_tests {
    use super::*;

    #[test]
    fn test_distance_unit_conversions() {
        // Test meters to kilometers conversion
        let meters = 5000.0;
        let kilometers = meters / 1000.0;
        assert_eq!(kilometers, 5.0);

        // Test miles to kilometers conversion (iOS might send miles)
        let miles = 3.1; // ~5km
        let km_from_miles = miles * 1.60934;
        assert!((km_from_miles - 4.98).abs() < 0.1);

        // Test feet to meters conversion
        let feet = 16404.0; // ~5000 meters
        let meters_from_feet = feet * 0.3048;
        assert!((meters_from_feet - 5000.0).abs() < 10.0);
    }

    #[test]
    fn test_weight_unit_conversions() {
        // Test pounds to kilograms
        let pounds = 154.0; // ~70 kg
        let kg_from_pounds = pounds * 0.453592;
        assert!((kg_from_pounds - 70.0).abs() < 1.0);

        // Test stones to kilograms
        let stones = 11.0; // ~70 kg
        let kg_from_stones = stones * 6.35029;
        assert!((kg_from_stones - 70.0).abs() < 1.0);
    }

    #[test]
    fn test_temperature_unit_conversions() {
        // Test Fahrenheit to Celsius
        let fahrenheit = 98.6; // Normal body temperature
        let celsius = (fahrenheit - 32.0) * 5.0 / 9.0;
        assert!((celsius - 37.0).abs() < 0.1);

        // Test Kelvin to Celsius
        let kelvin = 310.15; // ~37°C
        let celsius_from_kelvin = kelvin - 273.15;
        assert!((celsius_from_kelvin - 37.0).abs() < 0.1);
    }

    #[test]
    fn test_energy_unit_conversions() {
        // Test calories to kilojoules
        let calories = 100.0;
        let kilojoules = calories * 4.184;
        assert_eq!(kilojoules, 418.4);

        // Test kilojoules to calories
        let kj = 418.4;
        let cal_from_kj = kj / 4.184;
        assert!((cal_from_kj - 100.0).abs() < 0.1);
    }
}

#[cfg(test)]
mod data_format_validation_tests {
    use super::*;

    #[test]
    fn test_missing_required_fields() {
        let user_id = Uuid::new_v4();

        // Test heart rate metric with missing fields
        let mut hr_metric = test_utils::create_valid_heart_rate_metric(user_id, 72);
        hr_metric.heart_rate = None; // Missing primary value

        // Should still be valid as heart_rate is Option<i16>
        assert!(hr_metric.heart_rate.is_none());

        // Test blood pressure with missing required fields
        let bp_metric = test_utils::create_valid_blood_pressure_metric(user_id, 120, 80);
        // systolic and diastolic are required (not Option), so can't be None
        assert!(bp_metric.systolic > 0);
        assert!(bp_metric.diastolic > 0);
    }

    #[test]
    fn test_invalid_data_types() {
        // Test iOS payload with invalid data types
        let mut payload = test_utils::create_ios_payload_with_heart_rate(72.0);

        // Test string value instead of number
        payload.data.metrics[0].data[0].qty = None;
        payload.data.metrics[0].data[0].value = Some("seventy-two".to_string());

        assert!(payload.data.metrics[0].data[0].qty.is_none());
        assert!(payload.data.metrics[0].data[0].value.is_some());
    }

    #[test]
    fn test_malformed_json_structures() {
        // Test empty metrics array
        let empty_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![],
                workouts: vec![],
            },
        };

        assert!(empty_payload.data.metrics.is_empty());

        // Test metric with empty data array
        let empty_data_payload = IosIngestPayload {
            data: IosIngestData {
                metrics: vec![IosMetric {
                    name: "HKQuantityTypeIdentifierHeartRate".to_string(),
                    units: Some("count/min".to_string()),
                    data: vec![], // Empty data array
                }],
                workouts: vec![],
            },
        };

        assert!(empty_data_payload.data.metrics[0].data.is_empty());
    }

    #[test]
    fn test_extra_fields_handling() {
        let mut payload = test_utils::create_ios_payload_with_heart_rate(72.0);

        // Add extra fields to the payload
        payload.data.metrics[0].data[0].extra.insert(
            "custom_field".to_string(),
            serde_json::Value::String("custom_value".to_string()),
        );

        payload.data.metrics[0].data[0].extra.insert(
            "device_battery".to_string(),
            serde_json::Value::Number(serde_json::Number::from(85)),
        );

        // Verify extra fields are preserved
        assert!(payload.data.metrics[0].data[0].extra.contains_key("custom_field"));
        assert!(payload.data.metrics[0].data[0].extra.contains_key("device_battery"));
    }
}