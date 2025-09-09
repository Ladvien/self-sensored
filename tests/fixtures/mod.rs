use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use uuid::Uuid;

use self_sensored::models::{HealthMetric, Workout, IngestPayload, IngestData};

pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate a realistic heart rate value based on context
    pub fn heart_rate(context: &str) -> u16 {
        match context {
            "rest" => 60 + (rand::random::<u16>() % 20), // 60-80 BPM
            "exercise" => 140 + (rand::random::<u16>() % 60), // 140-200 BPM
            "sleep" => 45 + (rand::random::<u16>() % 20), // 45-65 BPM
            "stress" => 85 + (rand::random::<u16>() % 25), // 85-110 BPM
            "recovery" => 70 + (rand::random::<u16>() % 20), // 70-90 BPM
            _ => 70 + (rand::random::<u16>() % 30), // 70-100 BPM (default)
        }
    }

    /// Generate realistic blood pressure values
    pub fn blood_pressure() -> (u16, u16, Option<u16>) {
        let systolic = 110 + (rand::random::<u16>() % 40); // 110-150
        let diastolic = 60 + (rand::random::<u16>() % 30); // 60-90
        let pulse = Some(60 + (rand::random::<u16>() % 40)); // 60-100
        (systolic, diastolic, pulse)
    }

    /// Generate realistic sleep duration
    pub fn sleep_duration() -> u32 {
        360 + (rand::random::<u32>() % 240) // 6-10 hours in minutes
    }

    /// Generate realistic activity duration
    pub fn activity_duration() -> u32 {
        15 + (rand::random::<u32>() % 120) // 15-135 minutes
    }

    /// Generate realistic calorie values based on activity and duration
    pub fn calories_for_activity(activity: &str, duration_minutes: u32) -> i32 {
        let base_rate = match activity {
            "running" => 12, // calories per minute
            "cycling" => 8,
            "swimming" => 10,
            "walking" => 4,
            "weightlifting" => 6,
            "yoga" => 3,
            _ => 5,
        };
        base_rate * duration_minutes as i32
    }

    /// Generate realistic distance for activities
    pub fn distance_for_activity(activity: &str, duration_minutes: u32) -> Option<i32> {
        match activity {
            "running" => Some(duration_minutes as i32 * 200), // ~12 km/h
            "cycling" => Some(duration_minutes as i32 * 400), // ~24 km/h
            "swimming" => Some(duration_minutes as i32 * 50), // ~3 km/h
            "walking" => Some(duration_minutes as i32 * 80), // ~5 km/h
            _ => None,
        }
    }

    /// Generate a realistic GPS route
    pub fn gps_route(start_lat: f64, start_lon: f64, points: usize) -> Vec<(f64, f64)> {
        let mut route = Vec::new();
        let mut lat = start_lat;
        let mut lon = start_lon;

        for _ in 0..points {
            route.push((lat, lon));
            
            // Small random movement (roughly 50-200 meters per point)
            lat += (rand::random::<f64>() - 0.5) * 0.002;
            lon += (rand::random::<f64>() - 0.5) * 0.002;
            
            // Keep within reasonable bounds
            lat = lat.clamp(-90.0, 90.0);
            lon = lon.clamp(-180.0, 180.0);
        }

        route
    }
}

pub struct HealthMetricFixtures;

impl HealthMetricFixtures {
    pub fn heart_rate_rest() -> HealthMetric {
        HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: TestDataGenerator::heart_rate("rest"),
            context: Some("rest".to_string()),
            confidence: Some(0.95),
        }
    }

    pub fn heart_rate_exercise() -> HealthMetric {
        HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: TestDataGenerator::heart_rate("exercise"),
            context: Some("exercise".to_string()),
            confidence: Some(0.90),
        }
    }

    pub fn heart_rate_invalid_high() -> HealthMetric {
        HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 350, // Invalid
            context: Some("test".to_string()),
            confidence: Some(0.95),
        }
    }

    pub fn heart_rate_invalid_low() -> HealthMetric {
        HealthMetric::HeartRate {
            recorded_at: Utc::now(),
            heart_rate: 20, // Invalid
            context: Some("test".to_string()),
            confidence: Some(0.95),
        }
    }

    pub fn blood_pressure_normal() -> HealthMetric {
        let (systolic, diastolic, pulse) = TestDataGenerator::blood_pressure();
        HealthMetric::BloodPressure {
            recorded_at: Utc::now(),
            systolic,
            diastolic,
            pulse,
        }
    }

    pub fn blood_pressure_invalid() -> HealthMetric {
        HealthMetric::BloodPressure {
            recorded_at: Utc::now(),
            systolic: 80,  // Invalid: lower than diastolic
            diastolic: 120,
            pulse: Some(75),
        }
    }

    pub fn sleep_normal() -> HealthMetric {
        HealthMetric::Sleep {
            recorded_at: Utc::now(),
            duration_minutes: TestDataGenerator::sleep_duration(),
            sleep_stage: Some("deep".to_string()),
            efficiency: Some(0.80 + (rand::random::<f64>() * 0.15)), // 0.80-0.95
        }
    }

    pub fn sleep_invalid_duration() -> HealthMetric {
        HealthMetric::Sleep {
            recorded_at: Utc::now(),
            duration_minutes: 5, // Too short
            sleep_stage: Some("light".to_string()),
            efficiency: Some(0.85),
        }
    }

    pub fn activity_running() -> HealthMetric {
        let duration = TestDataGenerator::activity_duration();
        HealthMetric::Activity {
            recorded_at: Utc::now(),
            activity_type: "running".to_string(),
            duration_minutes: duration,
            calories_burned: Some(TestDataGenerator::calories_for_activity("running", duration)),
            distance_meters: TestDataGenerator::distance_for_activity("running", duration),
        }
    }

    pub fn activity_yoga() -> HealthMetric {
        let duration = 60 + (rand::random::<u32>() % 30); // 60-90 minutes
        HealthMetric::Activity {
            recorded_at: Utc::now(),
            activity_type: "yoga".to_string(),
            duration_minutes: duration,
            calories_burned: Some(TestDataGenerator::calories_for_activity("yoga", duration)),
            distance_meters: None, // Yoga doesn't have distance
        }
    }

    pub fn activity_invalid() -> HealthMetric {
        HealthMetric::Activity {
            recorded_at: Utc::now(),
            activity_type: "running".to_string(),
            duration_minutes: 0, // Invalid
            calories_burned: Some(100),
            distance_meters: Some(1000),
        }
    }

    /// Generate a batch of mixed realistic metrics
    pub fn mixed_batch(count: usize) -> Vec<HealthMetric> {
        let mut metrics = Vec::new();
        let base_time = Utc::now();

        for i in 0..count {
            let timestamp = base_time - chrono::Duration::minutes(i as i64);
            
            let metric = match i % 4 {
                0 => HealthMetric::HeartRate {
                    recorded_at: timestamp,
                    heart_rate: TestDataGenerator::heart_rate("rest"),
                    context: Some("rest".to_string()),
                    confidence: Some(0.95),
                },
                1 => {
                    let (systolic, diastolic, pulse) = TestDataGenerator::blood_pressure();
                    HealthMetric::BloodPressure {
                        recorded_at: timestamp,
                        systolic,
                        diastolic,
                        pulse,
                    }
                },
                2 => HealthMetric::Sleep {
                    recorded_at: timestamp,
                    duration_minutes: TestDataGenerator::sleep_duration(),
                    sleep_stage: Some("light".to_string()),
                    efficiency: Some(0.85),
                },
                3 => {
                    let duration = TestDataGenerator::activity_duration();
                    HealthMetric::Activity {
                        recorded_at: timestamp,
                        activity_type: "walking".to_string(),
                        duration_minutes: duration,
                        calories_burned: Some(TestDataGenerator::calories_for_activity("walking", duration)),
                        distance_meters: TestDataGenerator::distance_for_activity("walking", duration),
                    }
                },
                _ => unreachable!(),
            };
            
            metrics.push(metric);
        }

        metrics
    }
}

pub struct WorkoutFixtures;

impl WorkoutFixtures {
    pub fn running_workout() -> Workout {
        let duration = 30 + (rand::random::<i64>() % 60); // 30-90 minutes
        let start_time = Utc::now() - chrono::Duration::hours(1);
        let end_time = start_time + chrono::Duration::minutes(duration);

        Workout {
            workout_type: "running".to_string(),
            started_at: start_time,
            ended_at: end_time,
            duration_minutes: duration as u32,
            calories_burned: Some(TestDataGenerator::calories_for_activity("running", duration as u32)),
            distance_meters: TestDataGenerator::distance_for_activity("running", duration as u32),
            route_data: Some(TestDataGenerator::gps_route(37.7749, -122.4194, 20)), // San Francisco
        }
    }

    pub fn cycling_workout() -> Workout {
        let duration = 45 + (rand::random::<i64>() % 75); // 45-120 minutes
        let start_time = Utc::now() - chrono::Duration::hours(2);
        let end_time = start_time + chrono::Duration::minutes(duration);

        Workout {
            workout_type: "cycling".to_string(),
            started_at: start_time,
            ended_at: end_time,
            duration_minutes: duration as u32,
            calories_burned: Some(TestDataGenerator::calories_for_activity("cycling", duration as u32)),
            distance_meters: TestDataGenerator::distance_for_activity("cycling", duration as u32),
            route_data: Some(TestDataGenerator::gps_route(34.0522, -118.2437, 30)), // Los Angeles
        }
    }

    pub fn yoga_workout() -> Workout {
        let duration = 60 + (rand::random::<i64>() % 30); // 60-90 minutes
        let start_time = Utc::now() - chrono::Duration::hours(3);
        let end_time = start_time + chrono::Duration::minutes(duration);

        Workout {
            workout_type: "yoga".to_string(),
            started_at: start_time,
            ended_at: end_time,
            duration_minutes: duration as u32,
            calories_burned: Some(TestDataGenerator::calories_for_activity("yoga", duration as u32)),
            distance_meters: None,
            route_data: None, // Stationary workout
        }
    }

    pub fn invalid_workout() -> Workout {
        let start_time = Utc::now();
        let end_time = start_time - chrono::Duration::hours(1); // Invalid: ended before started

        Workout {
            workout_type: "running".to_string(),
            started_at: start_time,
            ended_at: end_time,
            duration_minutes: 60,
            calories_burned: Some(400),
            distance_meters: Some(8000),
            route_data: None,
        }
    }
}

pub struct PayloadFixtures;

impl PayloadFixtures {
    /// Create a standard format payload
    pub fn standard_payload(metrics: Vec<HealthMetric>, workouts: Vec<Workout>) -> IngestPayload {
        IngestPayload {
            device: Some("Test Device".to_string()),
            version: Some("1.0.0".to_string()),
            data: IngestData { metrics, workouts },
        }
    }

    /// Create an Auto Export iOS format payload
    pub fn ios_auto_export_payload() -> Value {
        json!({
            "data": [
                {
                    "type": "HeartRate",
                    "unit": "count/min",
                    "value": 75.0,
                    "date": "2024-01-15T10:30:00Z",
                    "source": "Apple Watch Series 9",
                    "metadata": {
                        "device": "Watch7,1",
                        "context": "Active",
                        "confidence": 0.95
                    }
                },
                {
                    "type": "BloodPressureSystolic",
                    "unit": "mmHg",
                    "value": 125.0,
                    "date": "2024-01-15T10:31:00Z",
                    "source": "Manual Entry"
                },
                {
                    "type": "BloodPressureDiastolic",
                    "unit": "mmHg",
                    "value": 82.0,
                    "date": "2024-01-15T10:31:00Z",
                    "source": "Manual Entry"
                },
                {
                    "type": "SleepAnalysis",
                    "unit": "min",
                    "value": 480.0,
                    "date": "2024-01-15T06:00:00Z",
                    "endDate": "2024-01-15T14:00:00Z",
                    "source": "iPhone",
                    "metadata": {
                        "stage": "deep",
                        "efficiency": 0.85
                    }
                },
                {
                    "type": "Workout",
                    "unit": "min",
                    "value": 45.0,
                    "date": "2024-01-15T10:00:00Z",
                    "endDate": "2024-01-15T10:45:00Z",
                    "source": "Apple Watch Series 9",
                    "metadata": {
                        "workoutType": "Running",
                        "totalDistance": 8000,
                        "totalEnergyBurned": 450,
                        "route": [
                            {"latitude": 37.7749, "longitude": -122.4194, "timestamp": "2024-01-15T10:00:00Z"},
                            {"latitude": 37.7849, "longitude": -122.4094, "timestamp": "2024-01-15T10:15:00Z"},
                            {"latitude": 37.7949, "longitude": -122.3994, "timestamp": "2024-01-15T10:30:00Z"},
                            {"latitude": 37.8049, "longitude": -122.3894, "timestamp": "2024-01-15T10:45:00Z"}
                        ]
                    }
                }
            ],
            "device": {
                "model": "iPhone15,2",
                "systemVersion": "17.2.1",
                "appVersion": "2.1.0"
            },
            "exportDate": "2024-01-15T15:00:00Z"
        })
    }

    /// Create a large iOS payload for performance testing
    pub fn large_ios_payload(days: usize) -> Value {
        let mut data_points = Vec::new();
        let base_date = chrono::Utc::now() - chrono::Duration::days(days as i64);

        // Generate heart rate data (every 15 minutes while awake)
        for day in 0..days {
            for hour in 6..22 { // Awake hours
                for quarter in 0..4 {
                    let timestamp = base_date + 
                        chrono::Duration::days(day as i64) + 
                        chrono::Duration::hours(hour) + 
                        chrono::Duration::minutes(quarter * 15);
                    
                    data_points.push(json!({
                        "type": "HeartRate",
                        "unit": "count/min",
                        "value": 70.0 + ((day * 16 + hour) % 40) as f64,
                        "date": timestamp.to_rfc3339(),
                        "source": "Apple Watch"
                    }));
                }
            }

            // Daily sleep data
            let sleep_start = base_date + 
                chrono::Duration::days(day as i64) + 
                chrono::Duration::hours(23);
            
            data_points.push(json!({
                "type": "SleepAnalysis",
                "unit": "min",
                "value": 480.0 + (day % 60) as f64,
                "date": sleep_start.to_rfc3339(),
                "endDate": (sleep_start + chrono::Duration::hours(8)).to_rfc3339(),
                "source": "iPhone"
            }));

            // Daily workout (alternating types)
            if day % 2 == 0 {
                let workout_start = base_date + 
                    chrono::Duration::days(day as i64) + 
                    chrono::Duration::hours(17);
                
                data_points.push(json!({
                    "type": "Workout",
                    "unit": "min", 
                    "value": 45.0,
                    "date": workout_start.to_rfc3339(),
                    "endDate": (workout_start + chrono::Duration::minutes(45)).to_rfc3339(),
                    "source": "Apple Watch",
                    "metadata": {
                        "workoutType": if day % 4 == 0 { "Running" } else { "Cycling" },
                        "totalDistance": if day % 4 == 0 { 8000 } else { 15000 },
                        "totalEnergyBurned": if day % 4 == 0 { 400 } else { 350 }
                    }
                }));
            }
        }

        json!({
            "data": data_points,
            "device": {
                "model": "iPhone15,2",
                "systemVersion": "17.2.1", 
                "appVersion": "2.1.0"
            },
            "exportDate": chrono::Utc::now().to_rfc3339()
        })
    }

    /// Create payload with validation errors
    pub fn invalid_payload() -> Value {
        json!({
            "data": [
                {
                    "type": "HeartRate",
                    "unit": "count/min",
                    "value": 400.0, // Invalid: too high
                    "date": "2024-01-15T10:30:00Z",
                    "source": "Test"
                },
                {
                    "type": "BloodPressureSystolic",
                    "unit": "mmHg",
                    "value": 300.0, // Invalid: too high
                    "date": "2024-01-15T10:31:00Z",
                    "source": "Test"
                }
            ]
        })
    }

    /// Create malformed JSON payload
    pub fn malformed_json() -> &'static str {
        r#"{"data": [{"type": "HeartRate", "value":}"#
    }

    /// Create payload that exceeds size limits
    pub fn oversized_payload() -> Value {
        let large_string = "x".repeat(10_000_000); // 10MB string
        json!({
            "data": [],
            "large_data": large_string
        })
    }
}

/// Helper functions for test setup
pub mod helpers {
    use sqlx::PgPool;
    use uuid::Uuid;
    use self_sensored::services::auth::AuthService;

    pub async fn create_test_user_with_key(
        pool: &PgPool,
        email: &str,
        key_name: &str,
        scopes: Vec<String>,
    ) -> (Uuid, String) {
        let auth_service = AuthService::new(pool.clone());

        // Clean up existing user
        sqlx::query!("DELETE FROM users WHERE email = $1", email)
            .execute(pool)
            .await
            .unwrap();

        // Create user
        let user = auth_service
            .create_user(email, Some("Test User"))
            .await
            .unwrap();

        // Create API key
        let (plain_key, _api_key) = auth_service
            .create_api_key(user.id, key_name, None, scopes)
            .await
            .unwrap();

        (user.id, plain_key)
    }

    pub async fn cleanup_test_user(pool: &PgPool, user_id: Uuid) {
        sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
            .execute(pool)
            .await
            .unwrap();
    }
}