use chrono::Utc;
use self_sensored::models::enums::{WorkoutCategory, WorkoutType};
use self_sensored::models::health_metrics::{RoutePoint, WorkoutData, WorkoutRoute};
use serde_json::json;
use uuid::Uuid;

/// Test comprehensive workout type support and categorization
#[test]
fn test_comprehensive_workout_types() {
    // Test base traditional activities
    assert_eq!(
        WorkoutType::from_ios_string("running"),
        WorkoutType::Running
    );
    assert_eq!(
        WorkoutType::from_ios_string("american_football"),
        WorkoutType::AmericanFootball
    );
    assert_eq!(
        WorkoutType::from_ios_string("football"),
        WorkoutType::AmericanFootball
    );
    assert_eq!(
        WorkoutType::from_ios_string("badminton"),
        WorkoutType::Badminton
    );
    assert_eq!(
        WorkoutType::from_ios_string("basketball"),
        WorkoutType::Basketball
    );
    assert_eq!(
        WorkoutType::from_ios_string("climbing"),
        WorkoutType::Climbing
    );
    assert_eq!(
        WorkoutType::from_ios_string("rock_climbing"),
        WorkoutType::Climbing
    );

    // Test iOS 10+ activities
    assert_eq!(WorkoutType::from_ios_string("hiit"), WorkoutType::Hiit);
    assert_eq!(
        WorkoutType::from_ios_string("high_intensity_interval_training"),
        WorkoutType::Hiit
    );
    assert_eq!(
        WorkoutType::from_ios_string("kickboxing"),
        WorkoutType::Kickboxing
    );
    assert_eq!(
        WorkoutType::from_ios_string("downhill_skiing"),
        WorkoutType::DownhillSkiing
    );
    assert_eq!(
        WorkoutType::from_ios_string("alpine_skiing"),
        WorkoutType::DownhillSkiing
    );

    // Test iOS 11+ activities
    assert_eq!(WorkoutType::from_ios_string("tai_chi"), WorkoutType::TaiChi);
    assert_eq!(
        WorkoutType::from_ios_string("mixed_cardio"),
        WorkoutType::MixedCardio
    );
    assert_eq!(
        WorkoutType::from_ios_string("hand_cycling"),
        WorkoutType::HandCycling
    );

    // Test iOS 13+ activities
    assert_eq!(
        WorkoutType::from_ios_string("disc_sports"),
        WorkoutType::DiscSports
    );
    assert_eq!(
        WorkoutType::from_ios_string("frisbee"),
        WorkoutType::DiscSports
    );
    assert_eq!(
        WorkoutType::from_ios_string("fitness_gaming"),
        WorkoutType::FitnessGaming
    );

    // Test legacy compatibility
    assert_eq!(
        WorkoutType::from_ios_string("strength_training"),
        WorkoutType::StrengthTraining
    );
    assert_eq!(
        WorkoutType::from_ios_string("weights"),
        WorkoutType::StrengthTraining
    );
    assert_eq!(WorkoutType::from_ios_string("sports"), WorkoutType::Sports);

    // Test unknown workout types default to Other
    assert_eq!(
        WorkoutType::from_ios_string("unknown_activity"),
        WorkoutType::Other
    );
    assert_eq!(WorkoutType::from_ios_string(""), WorkoutType::Other);
}

/// Test workout categorization for analytics and grouping
#[test]
fn test_workout_categorization() {
    // Test cardio category
    assert_eq!(WorkoutType::Running.category(), WorkoutCategory::Cardio);
    assert_eq!(WorkoutType::Cycling.category(), WorkoutCategory::Cardio);
    assert_eq!(
        WorkoutType::Swimming.category(),
        WorkoutCategory::WaterSports
    );
    assert_eq!(WorkoutType::Rowing.category(), WorkoutCategory::Cardio);
    assert_eq!(WorkoutType::Elliptical.category(), WorkoutCategory::Cardio);

    // Test strength training category
    assert_eq!(
        WorkoutType::StrengthTraining.category(),
        WorkoutCategory::StrengthTraining
    );
    assert_eq!(
        WorkoutType::TraditionalStrengthTraining.category(),
        WorkoutCategory::StrengthTraining
    );
    assert_eq!(
        WorkoutType::FunctionalStrengthTraining.category(),
        WorkoutCategory::StrengthTraining
    );

    // Test team sports category
    assert_eq!(
        WorkoutType::Basketball.category(),
        WorkoutCategory::TeamSports
    );
    assert_eq!(WorkoutType::Soccer.category(), WorkoutCategory::TeamSports);
    assert_eq!(
        WorkoutType::Volleyball.category(),
        WorkoutCategory::TeamSports
    );
    assert_eq!(
        WorkoutType::AmericanFootball.category(),
        WorkoutCategory::TeamSports
    );
    assert_eq!(WorkoutType::Hockey.category(), WorkoutCategory::TeamSports);

    // Test individual sports category
    assert_eq!(
        WorkoutType::Tennis.category(),
        WorkoutCategory::IndividualSports
    );
    assert_eq!(
        WorkoutType::Golf.category(),
        WorkoutCategory::IndividualSports
    );
    assert_eq!(
        WorkoutType::Boxing.category(),
        WorkoutCategory::IndividualSports
    );
    assert_eq!(
        WorkoutType::MartialArts.category(),
        WorkoutCategory::IndividualSports
    );

    // Test fitness classes category
    assert_eq!(
        WorkoutType::Yoga.category(),
        WorkoutCategory::FitnessClasses
    );
    assert_eq!(
        WorkoutType::Pilates.category(),
        WorkoutCategory::FitnessClasses
    );
    assert_eq!(
        WorkoutType::Hiit.category(),
        WorkoutCategory::FitnessClasses
    );
    assert_eq!(
        WorkoutType::Barre.category(),
        WorkoutCategory::FitnessClasses
    );
    assert_eq!(
        WorkoutType::Dance.category(),
        WorkoutCategory::FitnessClasses
    );

    // Test water sports category
    assert_eq!(
        WorkoutType::Swimming.category(),
        WorkoutCategory::WaterSports
    );
    assert_eq!(
        WorkoutType::WaterPolo.category(),
        WorkoutCategory::WaterSports
    );
    assert_eq!(
        WorkoutType::PaddleSports.category(),
        WorkoutCategory::WaterSports
    );
    assert_eq!(
        WorkoutType::SurfingSports.category(),
        WorkoutCategory::WaterSports
    );
    assert_eq!(
        WorkoutType::Sailing.category(),
        WorkoutCategory::WaterSports
    );

    // Test winter sports category
    assert_eq!(
        WorkoutType::SnowSports.category(),
        WorkoutCategory::WinterSports
    );
    assert_eq!(
        WorkoutType::DownhillSkiing.category(),
        WorkoutCategory::WinterSports
    );
    assert_eq!(
        WorkoutType::Snowboarding.category(),
        WorkoutCategory::WinterSports
    );

    // Test mind & body category
    assert_eq!(
        WorkoutType::MindAndBody.category(),
        WorkoutCategory::MindAndBody
    );
    assert_eq!(WorkoutType::TaiChi.category(), WorkoutCategory::MindAndBody);
    assert_eq!(
        WorkoutType::PreparationAndRecovery.category(),
        WorkoutCategory::MindAndBody
    );

    // Test accessibility category
    assert_eq!(
        WorkoutType::WheelchairWalkPace.category(),
        WorkoutCategory::Accessibility
    );
    assert_eq!(
        WorkoutType::WheelchairRunPace.category(),
        WorkoutCategory::Accessibility
    );
    assert_eq!(
        WorkoutType::HandCycling.category(),
        WorkoutCategory::Accessibility
    );

    // Test recreation category
    assert_eq!(WorkoutType::Play.category(), WorkoutCategory::Recreation);
    assert_eq!(WorkoutType::Fishing.category(), WorkoutCategory::Recreation);
    assert_eq!(
        WorkoutType::FitnessGaming.category(),
        WorkoutCategory::Recreation
    );
    assert_eq!(
        WorkoutType::DiscSports.category(),
        WorkoutCategory::Recreation
    );
}

/// Test workout type string formatting for database storage
#[test]
fn test_workout_type_formatting() {
    assert_eq!(
        WorkoutType::AmericanFootball.to_string(),
        "american_football"
    );
    assert_eq!(WorkoutType::DownhillSkiing.to_string(), "downhill_skiing");
    assert_eq!(
        WorkoutType::DanceInspiredTraining.to_string(),
        "dance_inspired_training"
    );
    assert_eq!(
        WorkoutType::MixedMetabolicCardioTraining.to_string(),
        "mixed_metabolic_cardio_training"
    );
    assert_eq!(
        WorkoutType::WheelchairRunPace.to_string(),
        "wheelchair_run_pace"
    );
    assert_eq!(WorkoutType::FitnessGaming.to_string(), "fitness_gaming");
    assert_eq!(WorkoutType::Other.to_string(), "other");
}

/// Test GPS route point validation
#[test]
fn test_route_point_validation() {
    let now = Utc::now();
    let valid_points = vec![
        RoutePoint {
            latitude: 40.7128,
            longitude: -74.0060,
            timestamp: now,
            altitude: Some(10.0),
            accuracy: Some(5.0),
            speed: Some(2.5),
        },
        RoutePoint {
            latitude: 40.7129,
            longitude: -74.0059,
            timestamp: now + chrono::Duration::seconds(10),
            altitude: Some(11.0),
            accuracy: Some(4.0),
            speed: Some(3.0),
        },
    ];

    // Valid route should pass validation
    assert!(WorkoutRoute::validate_route_points(&valid_points).is_ok());

    // Empty route should fail
    let empty_points = vec![];
    assert!(WorkoutRoute::validate_route_points(&empty_points).is_err());

    // Invalid latitude should fail
    let invalid_lat_points = vec![RoutePoint {
        latitude: 91.0, // Invalid latitude
        longitude: -74.0060,
        timestamp: now,
        altitude: None,
        accuracy: None,
        speed: None,
    }];
    assert!(WorkoutRoute::validate_route_points(&invalid_lat_points).is_err());

    // Invalid longitude should fail
    let invalid_lon_points = vec![RoutePoint {
        latitude: 40.7128,
        longitude: 181.0, // Invalid longitude
        timestamp: now,
        altitude: None,
        accuracy: None,
        speed: None,
    }];
    assert!(WorkoutRoute::validate_route_points(&invalid_lon_points).is_err());

    // Unrealistic altitude should fail
    let invalid_alt_points = vec![RoutePoint {
        latitude: 40.7128,
        longitude: -74.0060,
        timestamp: now,
        altitude: Some(10000.0), // Too high
        accuracy: None,
        speed: None,
    }];
    assert!(WorkoutRoute::validate_route_points(&invalid_alt_points).is_err());

    // Unrealistic speed should fail
    let invalid_speed_points = vec![RoutePoint {
        latitude: 40.7128,
        longitude: -74.0060,
        timestamp: now,
        altitude: None,
        accuracy: None,
        speed: Some(200.0), // Too fast (200 m/s = 447 mph)
    }];
    assert!(WorkoutRoute::validate_route_points(&invalid_speed_points).is_err());

    // Invalid GPS accuracy should fail
    let invalid_accuracy_points = vec![RoutePoint {
        latitude: 40.7128,
        longitude: -74.0060,
        timestamp: now,
        altitude: None,
        accuracy: Some(1500.0), // Too inaccurate
        speed: None,
    }];
    assert!(WorkoutRoute::validate_route_points(&invalid_accuracy_points).is_err());

    // Out of order timestamps should fail
    let out_of_order_points = vec![
        RoutePoint {
            latitude: 40.7128,
            longitude: -74.0060,
            timestamp: now + chrono::Duration::seconds(10),
            altitude: None,
            accuracy: None,
            speed: None,
        },
        RoutePoint {
            latitude: 40.7129,
            longitude: -74.0059,
            timestamp: now, // Earlier than previous point
            altitude: None,
            accuracy: None,
            speed: None,
        },
    ];
    assert!(WorkoutRoute::validate_route_points(&out_of_order_points).is_err());
}

/// Test GPS route metrics calculation
#[test]
fn test_route_metrics_calculation() {
    let now = Utc::now();
    let points = vec![
        RoutePoint {
            latitude: 40.7128, // NYC coordinates
            longitude: -74.0060,
            timestamp: now,
            altitude: Some(10.0),
            accuracy: Some(5.0),
            speed: Some(0.0),
        },
        RoutePoint {
            latitude: 40.7129, // Slightly north
            longitude: -74.0060,
            timestamp: now + chrono::Duration::seconds(30),
            altitude: Some(15.0), // 5m elevation gain
            accuracy: Some(4.0),
            speed: Some(2.0),
        },
        RoutePoint {
            latitude: 40.7130, // Further north
            longitude: -74.0060,
            timestamp: now + chrono::Duration::seconds(60),
            altitude: Some(12.0), // 3m elevation loss
            accuracy: Some(6.0),
            speed: Some(1.5),
        },
    ];

    let metrics = WorkoutRoute::calculate_metrics_from_points(&points);

    // Should have calculated distance
    assert!(metrics.total_distance_meters > 0.0);
    assert!(metrics.total_distance_meters < 1000.0); // Reasonable for small coordinate changes

    // Should have elevation changes
    assert_eq!(metrics.elevation_gain_meters, 5.0);
    assert_eq!(metrics.elevation_loss_meters, 3.0);

    // Should have altitude bounds
    assert_eq!(metrics.max_altitude_meters, Some(15.0));
    assert_eq!(metrics.min_altitude_meters, Some(10.0));

    // Should have average GPS accuracy
    let expected_accuracy = (5.0 + 4.0 + 6.0) / 3.0;
    assert_eq!(metrics.average_accuracy_meters, Some(expected_accuracy));

    // Test empty points
    let empty_metrics = WorkoutRoute::calculate_metrics_from_points(&[]);
    assert_eq!(empty_metrics.total_distance_meters, 0.0);
    assert_eq!(empty_metrics.elevation_gain_meters, 0.0);
    assert_eq!(empty_metrics.elevation_loss_meters, 0.0);
    assert_eq!(empty_metrics.max_altitude_meters, None);
    assert_eq!(empty_metrics.min_altitude_meters, None);
    assert_eq!(empty_metrics.average_accuracy_meters, None);
}

/// Test GPS distance calculation using Haversine formula
#[test]
fn test_haversine_distance_calculation() {
    let now = Utc::now();

    // Test known distance: NYC to Philadelphia is approximately 129 km
    let nyc_to_philly = vec![
        RoutePoint {
            latitude: 40.7128, // NYC
            longitude: -74.0060,
            timestamp: now,
            altitude: None,
            accuracy: None,
            speed: None,
        },
        RoutePoint {
            latitude: 39.9526, // Philadelphia
            longitude: -75.1652,
            timestamp: now + chrono::Duration::hours(2),
            altitude: None,
            accuracy: None,
            speed: None,
        },
    ];

    let metrics = WorkoutRoute::calculate_metrics_from_points(&nyc_to_philly);
    // Should be approximately 129 km (allow 10% tolerance for precision)
    assert!(metrics.total_distance_meters > 116_000.0); // 129km - 10%
    assert!(metrics.total_distance_meters < 142_000.0); // 129km + 10%

    // Test same point (should be 0 distance)
    let same_point = vec![
        RoutePoint {
            latitude: 40.7128,
            longitude: -74.0060,
            timestamp: now,
            altitude: None,
            accuracy: None,
            speed: None,
        },
        RoutePoint {
            latitude: 40.7128,
            longitude: -74.0060,
            timestamp: now + chrono::Duration::seconds(10),
            altitude: None,
            accuracy: None,
            speed: None,
        },
    ];

    let same_metrics = WorkoutRoute::calculate_metrics_from_points(&same_point);
    assert!(same_metrics.total_distance_meters < 1.0); // Should be essentially 0
}

/// Test comprehensive workout data structure
#[test]
fn test_workout_data_structure() {
    let workout_id = Uuid::new_v4();
    let user_id = Uuid::new_v4();
    let started_at = Utc::now() - chrono::Duration::hours(1);
    let ended_at = Utc::now();

    let workout_data = WorkoutData {
        id: workout_id,
        user_id,
        workout_type: WorkoutType::Running,
        started_at,
        ended_at,
        total_energy_kcal: Some(300.0),
        active_energy_kcal: Some(250.0),
        distance_meters: Some(5000.0),
        avg_heart_rate: Some(150),
        max_heart_rate: Some(180),
        source_device: Some("Apple Watch".to_string()),
        created_at: Utc::now(),
    };

    // Test that workout data structure is properly formed
    assert_eq!(workout_data.workout_type, WorkoutType::Running);
    assert_eq!(
        workout_data.workout_type.category(),
        WorkoutCategory::Cardio
    );
    assert!(workout_data.total_energy_kcal.is_some());
    assert!(workout_data.distance_meters.is_some());
}

/// Test workout route JSON serialization/deserialization
#[test]
fn test_route_json_serialization() {
    let now = Utc::now();
    let points = vec![
        RoutePoint {
            latitude: 40.7128,
            longitude: -74.0060,
            timestamp: now,
            altitude: Some(10.0),
            accuracy: Some(5.0),
            speed: Some(2.5),
        },
        RoutePoint {
            latitude: 40.7129,
            longitude: -74.0059,
            timestamp: now + chrono::Duration::seconds(10),
            altitude: Some(11.0),
            accuracy: Some(4.0),
            speed: Some(3.0),
        },
    ];

    // Test JSON serialization
    let json_value = json!(points);
    let serialized = serde_json::to_string(&json_value).unwrap();
    assert!(serialized.contains("latitude"));
    assert!(serialized.contains("longitude"));
    assert!(serialized.contains("timestamp"));
    assert!(serialized.contains("altitude"));
    assert!(serialized.contains("accuracy"));
    assert!(serialized.contains("speed"));

    // Test deserialization
    let deserialized: Vec<RoutePoint> = serde_json::from_str(&serialized).unwrap();
    assert_eq!(deserialized.len(), 2);
    assert_eq!(deserialized[0].latitude, 40.7128);
    assert_eq!(deserialized[1].longitude, -74.0059);
    assert_eq!(deserialized[1].altitude, Some(11.0));
}

/// Test multi-sport workout type support
#[test]
fn test_multi_sport_scenarios() {
    // Test triathlon component sports
    assert_eq!(
        WorkoutType::Swimming.category(),
        WorkoutCategory::WaterSports
    );
    assert_eq!(WorkoutType::Cycling.category(), WorkoutCategory::Cardio);
    assert_eq!(WorkoutType::Running.category(), WorkoutCategory::Cardio);

    // Test winter sports diversity
    assert_eq!(
        WorkoutType::DownhillSkiing.category(),
        WorkoutCategory::WinterSports
    );
    assert_eq!(
        WorkoutType::CrossCountrySkiing.category(),
        WorkoutCategory::Cardio
    );
    assert_eq!(
        WorkoutType::Snowboarding.category(),
        WorkoutCategory::WinterSports
    );

    // Test accessibility sports
    assert_eq!(
        WorkoutType::WheelchairRunPace.category(),
        WorkoutCategory::Accessibility
    );
    assert_eq!(
        WorkoutType::WheelchairWalkPace.category(),
        WorkoutCategory::Accessibility
    );
    assert_eq!(
        WorkoutType::HandCycling.category(),
        WorkoutCategory::Accessibility
    );

    // Test combat sports
    assert_eq!(
        WorkoutType::Boxing.category(),
        WorkoutCategory::IndividualSports
    );
    assert_eq!(
        WorkoutType::MartialArts.category(),
        WorkoutCategory::IndividualSports
    );
    assert_eq!(
        WorkoutType::Wrestling.category(),
        WorkoutCategory::IndividualSports
    );

    // Test dance and fitness
    assert_eq!(
        WorkoutType::Dance.category(),
        WorkoutCategory::FitnessClasses
    );
    assert_eq!(
        WorkoutType::DanceInspiredTraining.category(),
        WorkoutCategory::FitnessClasses
    );
    assert_eq!(
        WorkoutType::Barre.category(),
        WorkoutCategory::FitnessClasses
    );
}
