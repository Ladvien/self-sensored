// Integration tests for hygiene events API endpoints
use actix_web::{test, web, App, HttpMessage};
use chrono::{Duration, Utc};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

// Import modules from the main application
use self_sensored::{
    config::ValidationConfig,
    handlers::hygiene_handler::{
        get_hygiene_data, ingest_hygiene, HygieneIngestPayload, HygieneIngestRequest,
    },
    services::{auth::AuthContext, batch_processor::BatchProcessor},
};

/// Test helper to create a test user and API key
async fn create_test_user_and_api_key(pool: &PgPool) -> (Uuid, String) {
    let user_id = Uuid::new_v4();
    let api_key_id = Uuid::new_v4();
    let test_email = format!(
        "hygiene_test_{}@example.com",
        user_id.to_string().replace('-', "")
    );

    // Create test user
    sqlx::query!(
        "INSERT INTO users (id, email, created_at) VALUES ($1, $2, NOW())",
        user_id,
        test_email
    )
    .execute(pool)
    .await
    .expect("Failed to create test user");

    // Create API key (using a simple hash for testing)
    let api_key = format!(
        "test_hygiene_api_key_{}",
        api_key_id.to_string().replace('-', "")
    );
    let api_key_hash = format!("hash_{}", api_key);

    sqlx::query!(
        r#"INSERT INTO api_keys (id, user_id, key_hash, name, created_at, is_active)
           VALUES ($1, $2, $3, 'Test Hygiene API Key', NOW(), true)"#,
        api_key_id,
        user_id,
        api_key_hash
    )
    .execute(pool)
    .await
    .expect("Failed to create test API key");

    (user_id, api_key)
}

/// Clean up test data
async fn cleanup_test_data(pool: &PgPool, user_id: Uuid) {
    // Delete in reverse dependency order
    sqlx::query!("DELETE FROM hygiene_events WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM api_keys WHERE user_id = $1", user_id)
        .execute(pool)
        .await
        .ok();

    sqlx::query!("DELETE FROM users WHERE id = $1", user_id)
        .execute(pool)
        .await
        .ok();
}

/// Create sample hygiene events for testing
fn create_sample_hygiene_events(count: usize) -> Vec<HygieneIngestRequest> {
    let mut events = Vec::new();
    let base_time = Utc::now() - Duration::hours(24);

    for i in 0..count {
        let event_time = base_time + Duration::hours(i as i64);

        // Alternate between different hygiene event types
        let (event_type, duration, meets_guidelines) = match i % 4 {
            0 => ("handwashing", Some(25), Some(true)), // Good handwashing
            1 => ("handwashing", Some(10), Some(false)), // Poor handwashing
            2 => ("toothbrushing", Some(130), Some(true)), // Good toothbrushing
            3 => ("toothbrushing", Some(60), Some(false)), // Poor toothbrushing
            _ => ("hand_sanitizer", Some(15), Some(true)),
        };

        events.push(HygieneIngestRequest {
            recorded_at: event_time,
            event_type: event_type.to_string(),
            duration_seconds: duration,
            quality_rating: Some((i % 5 + 1) as i16), // Rotate 1-5
            meets_who_guidelines: meets_guidelines,
            frequency_compliance_rating: Some((i % 5 + 1) as i16),
            device_detected: Some(i % 2 == 0), // Alternate device detection
            device_effectiveness_score: Some((i % 10 + 1) as f64 * 10.0), // 10-100%
            trigger_event: Some(match i % 3 {
                0 => "routine".to_string(),
                1 => "after_bathroom".to_string(),
                _ => "before_meal".to_string(),
            }),
            location_context: Some("home".to_string()),
            compliance_motivation: Some("personal_hygiene".to_string()),
            health_crisis_enhanced: Some(i % 10 == 0), // 10% during health crisis
            crisis_compliance_level: if i % 10 == 0 { Some(5) } else { None },
            daily_goal_progress: Some(((i % 10 + 1) * 10) as i16), // 10-100%
            achievement_unlocked: if i % 20 == 0 {
                Some("Perfect Week Hygiene".to_string())
            } else {
                None
            },
            medication_adherence_related: Some(i % 15 == 0), // Some medical context
            medical_condition_context: if i % 15 == 0 {
                Some("diabetes_management".to_string())
            } else {
                None
            },
            data_sensitivity_level: Some("standard".to_string()),
            source_device: Some(format!("test_device_{}", i % 3)),
        });
    }

    events
}

#[tokio::test]
async fn test_hygiene_events_ingest_comprehensive() {
    println!("🧼 Starting comprehensive hygiene events ingestion test");

    // Get test database connection
    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Create test user and API key
    let (user_id, _api_key) = create_test_user_and_api_key(&pool).await;

    // Initialize dependencies
    let validation_config = ValidationConfig::default();
    let batch_processor = BatchProcessor::new(pool.clone());

    // Create sample hygiene events
    let hygiene_events = create_sample_hygiene_events(50);
    let payload = HygieneIngestPayload { hygiene_events };

    // Create mock auth context
    let auth = AuthContext::new_for_testing(user_id);

    // Create test app
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(batch_processor))
            .app_data(web::Data::new(validation_config))
            .service(web::resource("/ingest/hygiene").route(web::post().to(ingest_hygiene))),
    )
    .await;

    // Test hygiene events ingestion
    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    // Add auth context to request extensions
    let mut req = req;
    req.extensions_mut().insert(auth);

    let resp = test::call_service(&app, req).await;

    println!("📊 Ingestion response status: {}", resp.status());
    assert!(
        resp.status().is_success(),
        "Hygiene events ingestion should succeed"
    );

    let response_body: Value = test::read_body_json(resp).await;
    println!(
        "📈 Response body: {}",
        serde_json::to_string_pretty(&response_body).unwrap()
    );

    // Verify response structure
    assert!(response_body["success"].as_bool().unwrap_or(false));
    assert!(response_body["processed_count"].as_u64().unwrap() > 0);
    assert!(response_body.get("hygiene_analysis").is_some());

    // Verify data was stored in database - simple count check
    let stored_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM hygiene_events WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count stored hygiene events");

    println!(
        "🏥 Stored {} hygiene events in database",
        stored_count.unwrap_or(0)
    );
    assert!(stored_count.unwrap_or(0) > 0);

    // Test specific hygiene analysis features
    let analysis = response_body["hygiene_analysis"].as_object().unwrap();

    // Check compliance scores
    assert!(analysis.get("compliance_score").is_some());
    assert!(analysis.get("handwashing_compliance").is_some());
    assert!(analysis.get("toothbrushing_compliance").is_some());

    // Check public health insights
    let public_health = analysis["public_health_insights"].as_object().unwrap();
    assert!(public_health.get("infection_prevention_score").is_some());
    assert!(public_health.get("risk_level").is_some());
    assert!(public_health["recommended_improvements"].is_array());

    // Check habit strength analysis
    let habit_summary = analysis["habit_strength_summary"].as_object().unwrap();
    assert!(habit_summary.get("average_streak_length").is_some());

    println!("✅ Comprehensive hygiene events ingestion test completed successfully");

    // Cleanup
    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_hygiene_data_retrieval_with_filters() {
    println!("📋 Starting hygiene data retrieval with filters test");

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let (user_id, _api_key) = create_test_user_and_api_key(&pool).await;

    // Insert test hygiene data directly
    let test_events = vec![
        ("handwashing", 25, true, false),
        ("handwashing", 15, false, false),
        ("toothbrushing", 120, true, true), // During health crisis
        ("hand_sanitizer", 12, false, false),
        ("face_washing", 30, true, false),
    ];

    for (i, (event_type, duration, meets_guidelines, crisis)) in test_events.iter().enumerate() {
        let event_time = Utc::now() - Duration::hours((i + 1) as i64);

        sqlx::query!(
            r#"INSERT INTO hygiene_events (
                user_id, recorded_at, event_type, duration_seconds,
                quality_rating, meets_who_guidelines, health_crisis_enhanced,
                source_device
            ) VALUES ($1, $2, $3::text::hygiene_event_type, $4, $5, $6, $7, $8)"#,
            user_id,
            event_time,
            event_type,
            duration,
            3_i16,
            meets_guidelines,
            crisis,
            "test_device"
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test hygiene event");
    }

    let auth = AuthContext::new_for_testing(user_id);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(web::resource("/data/hygiene").route(web::get().to(get_hygiene_data))),
    )
    .await;

    // Test 1: Get all hygiene data
    let req = test::TestRequest::get().uri("/data/hygiene").to_request();

    let mut req = req;
    req.extensions_mut().insert(auth.clone());

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: Value = test::read_body_json(resp).await;
    println!(
        "📊 All events response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    assert!(response["hygiene_events"].is_array());
    assert!(response["total_count"].as_i64().unwrap() > 0);
    assert!(response.get("compliance_summary").is_some());

    // Test 2: Filter by event type (handwashing only)
    let req = test::TestRequest::get()
        .uri("/data/hygiene?event_type=handwashing")
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth.clone());

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: Value = test::read_body_json(resp).await;
    let events = response["hygiene_events"].as_array().unwrap();

    // Should only return handwashing events
    for event in events {
        assert_eq!(event["event_type"].as_str().unwrap(), "handwashing");
    }

    // Test 3: Filter for compliance only
    let req = test::TestRequest::get()
        .uri("/data/hygiene?compliance_only=true")
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth.clone());

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: Value = test::read_body_json(resp).await;
    let events = response["hygiene_events"].as_array().unwrap();

    // Should only return WHO guideline compliant events
    for event in events {
        assert_eq!(
            event["meets_who_guidelines"].as_bool().unwrap_or(false),
            true
        );
    }

    // Test 4: Filter for health crisis period
    let req = test::TestRequest::get()
        .uri("/data/hygiene?crisis_period=true")
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: Value = test::read_body_json(resp).await;
    let events = response["hygiene_events"].as_array().unwrap();

    // Should only return health crisis events
    for event in events {
        assert_eq!(
            event["health_crisis_enhanced"].as_bool().unwrap_or(false),
            true
        );
    }

    println!("✅ Hygiene data retrieval with filters test completed successfully");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_hygiene_events_validation_and_error_handling() {
    println!("⚠️ Starting hygiene events validation and error handling test");

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let (user_id, _api_key) = create_test_user_and_api_key(&pool).await;
    let validation_config = ValidationConfig::default();
    let batch_processor = BatchProcessor::new(pool.clone());
    let auth = AuthContext::new_for_testing(user_id);

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(batch_processor))
            .app_data(web::Data::new(validation_config))
            .service(web::resource("/ingest/hygiene").route(web::post().to(ingest_hygiene))),
    )
    .await;

    // Test 1: Invalid event type
    let invalid_payload = json!({
        "hygiene_events": [
            {
                "recorded_at": Utc::now(),
                "event_type": "invalid_event_type",
                "duration_seconds": 30
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_payload)
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth.clone());

    let resp = test::call_service(&app, req).await;
    let response: Value = test::read_body_json(resp).await;

    println!(
        "🚫 Invalid event type response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    assert_eq!(response["success"].as_bool().unwrap(), false);
    assert!(response["errors"].as_array().unwrap().len() > 0);

    // Test 2: Invalid duration (negative)
    let invalid_duration_payload = json!({
        "hygiene_events": [
            {
                "recorded_at": Utc::now(),
                "event_type": "handwashing",
                "duration_seconds": -10,
                "quality_rating": 3
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_duration_payload)
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth.clone());

    let resp = test::call_service(&app, req).await;
    let response: Value = test::read_body_json(resp).await;

    println!(
        "⏱️ Invalid duration response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    assert_eq!(response["success"].as_bool().unwrap(), false);

    // Test 3: Invalid quality rating (out of range)
    let invalid_rating_payload = json!({
        "hygiene_events": [
            {
                "recorded_at": Utc::now(),
                "event_type": "toothbrushing",
                "duration_seconds": 120,
                "quality_rating": 10  // Should be 1-5
            }
        ]
    });

    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&invalid_rating_payload)
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth.clone());

    let resp = test::call_service(&app, req).await;
    let response: Value = test::read_body_json(resp).await;

    println!(
        "⭐ Invalid quality rating response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );
    assert_eq!(response["success"].as_bool().unwrap(), false);

    // Test 4: Empty payload
    let empty_payload = json!({
        "hygiene_events": []
    });

    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&empty_payload)
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth);

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400); // Bad Request

    let response: Value = test::read_body_json(resp).await;
    assert_eq!(response["error"].as_str().unwrap(), "empty_payload");

    println!("✅ Hygiene events validation and error handling test completed successfully");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_hygiene_compliance_analysis() {
    println!("📈 Starting hygiene compliance analysis test");

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let (user_id, _api_key) = create_test_user_and_api_key(&pool).await;

    // Create specific test events for compliance testing
    let compliance_events = vec![
        HygieneIngestRequest {
            recorded_at: Utc::now() - Duration::hours(1),
            event_type: "handwashing".to_string(),
            duration_seconds: Some(25), // Meets WHO guideline (20+ seconds)
            quality_rating: Some(5),
            meets_who_guidelines: Some(true),
            device_detected: Some(true),
            device_effectiveness_score: Some(95.0),
            trigger_event: Some("after_bathroom".to_string()),
            location_context: Some("home".to_string()),
            compliance_motivation: Some("infection_prevention".to_string()),
            health_crisis_enhanced: Some(true),
            crisis_compliance_level: Some(5),
            daily_goal_progress: Some(100),
            achievement_unlocked: Some("Perfect Handwashing".to_string()),
            medication_adherence_related: Some(false),
            medical_condition_context: None,
            data_sensitivity_level: Some("standard".to_string()),
            source_device: Some("smart_soap_dispenser".to_string()),
            frequency_compliance_rating: Some(5),
        },
        HygieneIngestRequest {
            recorded_at: Utc::now() - Duration::minutes(30),
            event_type: "toothbrushing".to_string(),
            duration_seconds: Some(140), // Meets ADA guideline (120+ seconds)
            quality_rating: Some(4),
            meets_who_guidelines: Some(true),
            device_detected: Some(true),
            device_effectiveness_score: Some(88.0),
            trigger_event: Some("routine".to_string()),
            location_context: Some("home".to_string()),
            compliance_motivation: Some("dental_health".to_string()),
            health_crisis_enhanced: Some(false),
            crisis_compliance_level: None,
            daily_goal_progress: Some(80),
            achievement_unlocked: None,
            medication_adherence_related: Some(false),
            medical_condition_context: None,
            data_sensitivity_level: Some("standard".to_string()),
            source_device: Some("smart_toothbrush".to_string()),
            frequency_compliance_rating: Some(4),
        },
    ];

    let validation_config = ValidationConfig::default();
    let batch_processor = BatchProcessor::new(pool.clone());
    let auth = AuthContext::new_for_testing(user_id);

    let payload = HygieneIngestPayload {
        hygiene_events: compliance_events,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(batch_processor))
            .app_data(web::Data::new(validation_config))
            .service(web::resource("/ingest/hygiene").route(web::post().to(ingest_hygiene))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: Value = test::read_body_json(resp).await;
    println!(
        "🏥 Compliance analysis response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Verify compliance analysis
    let analysis = response["hygiene_analysis"].as_object().unwrap();

    // Both events should meet guidelines, so compliance should be high
    let compliance_score = analysis["compliance_score"].as_f64().unwrap();
    assert!(
        compliance_score >= 80.0,
        "Compliance score should be high for guideline-meeting events"
    );

    let handwashing_compliance = analysis["handwashing_compliance"].as_f64().unwrap();
    assert_eq!(
        handwashing_compliance, 100.0,
        "Handwashing compliance should be 100%"
    );

    let toothbrushing_compliance = analysis["toothbrushing_compliance"].as_f64().unwrap();
    assert_eq!(
        toothbrushing_compliance, 100.0,
        "Toothbrushing compliance should be 100%"
    );

    // Check critical hygiene events (handwashing)
    let critical_events = analysis["critical_hygiene_events"].as_u64().unwrap();
    assert_eq!(
        critical_events, 1,
        "Should have 1 critical hygiene event (handwashing)"
    );

    // Check smart device detections
    let smart_device_detections = analysis["smart_device_detections"].as_u64().unwrap();
    assert_eq!(
        smart_device_detections, 2,
        "Both events were detected by smart devices"
    );

    // Check health crisis events
    let health_crisis_events = analysis["health_crisis_events"].as_u64().unwrap();
    assert_eq!(health_crisis_events, 1, "Should have 1 health crisis event");

    // Check public health insights
    let public_health = analysis["public_health_insights"].as_object().unwrap();
    let infection_prevention_score = public_health["infection_prevention_score"]
        .as_f64()
        .unwrap();
    assert!(
        infection_prevention_score > 40.0,
        "Infection prevention score should be reasonable"
    );

    let risk_level = public_health["risk_level"].as_str().unwrap();
    assert!(
        matches!(risk_level, "low" | "moderate"),
        "Risk level should be low or moderate for compliant events"
    );

    println!("✅ Hygiene compliance analysis test completed successfully");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_hygiene_public_health_tracking() {
    println!("🌍 Starting public health tracking test for hygiene events");

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let (user_id, _api_key) = create_test_user_and_api_key(&pool).await;

    // Test hygiene compliance scoring functions directly from database
    let compliance_result = sqlx::query!(
        "SELECT calculate_hygiene_compliance_score($1) as compliance_score",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to call compliance function");

    println!(
        "📊 Initial compliance score: {:?}",
        compliance_result.compliance_score
    );

    // Insert test data that simulates public health scenarios
    let public_health_events = vec![
        // Crisis period handwashing (enhanced compliance)
        ("handwashing", 25, true, true, "crisis_protocol", 5),
        ("handwashing", 30, true, true, "crisis_protocol", 5),
        ("hand_sanitizer", 15, true, true, "crisis_protocol", 4),
        // Regular period with mixed compliance
        ("handwashing", 15, false, false, "routine", 3),
        ("toothbrushing", 90, false, false, "routine", 2),
        ("toothbrushing", 130, true, false, "routine", 4),
    ];

    for (i, (event_type, duration, meets_guidelines, crisis, trigger, quality)) in
        public_health_events.iter().enumerate()
    {
        let event_time = Utc::now() - Duration::hours((i + 1) as i64);

        sqlx::query!(
            r#"INSERT INTO hygiene_events (
                user_id, recorded_at, event_type, duration_seconds, quality_rating,
                meets_who_guidelines, health_crisis_enhanced, trigger_event,
                crisis_compliance_level, source_device
            ) VALUES ($1, $2, $3::text::hygiene_event_type, $4, $5, $6, $7, $8, $9, 'public_health_test')"#,
            user_id,
            event_time,
            event_type,
            duration,
            *quality as i16,
            meets_guidelines,
            crisis,
            trigger,
            if *crisis { Some(5_i16) } else { None }
        )
        .execute(&pool)
        .await
        .expect("Failed to insert public health test event");
    }

    // Test compliance calculation after inserting data
    let updated_compliance = sqlx::query!(
        "SELECT calculate_hygiene_compliance_score($1) as compliance_score",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to get updated compliance score");

    println!(
        "📈 Updated compliance score: {:?}",
        updated_compliance.compliance_score
    );

    if let Some(compliance_json) = updated_compliance.compliance_score {
        let compliance: serde_json::Value =
            serde_json::from_value(compliance_json).expect("Failed to parse compliance JSON");

        println!(
            "🔍 Compliance breakdown: {}",
            serde_json::to_string_pretty(&compliance).unwrap()
        );

        // Verify compliance metrics
        assert!(compliance.get("overall_score").is_some());
        assert!(compliance.get("handwashing_compliance_percent").is_some());
        assert!(compliance.get("toothbrushing_compliance_percent").is_some());
        assert!(compliance.get("daily_frequency").is_some());

        let overall_score = compliance["overall_score"].as_f64().unwrap();
        assert!(overall_score >= 0.0 && overall_score <= 100.0);
    }

    // Test streak functionality
    let streak_events = vec![
        ("handwashing", Utc::now() - Duration::days(3)),
        ("handwashing", Utc::now() - Duration::days(2)),
        ("handwashing", Utc::now() - Duration::days(1)),
        ("handwashing", Utc::now()),
    ];

    for (event_type, timestamp) in streak_events {
        sqlx::query!(
            r#"INSERT INTO hygiene_events (
                user_id, recorded_at, event_type, duration_seconds,
                meets_who_guidelines, source_device
            ) VALUES ($1, $2, $3::text::hygiene_event_type, 25, true, 'streak_test')"#,
            user_id,
            timestamp,
            event_type
        )
        .execute(&pool)
        .await
        .expect("Failed to insert streak test event");
    }

    // Verify streak calculation
    let streak_result = sqlx::query!(
        "SELECT MAX(streak_count) as max_streak FROM hygiene_events WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to get streak data");

    println!("🔥 Maximum streak count: {:?}", streak_result.max_streak);
    assert!(streak_result.max_streak.unwrap_or(0) > 0);

    println!("✅ Public health tracking test completed successfully");

    cleanup_test_data(&pool, user_id).await;
}

#[tokio::test]
async fn test_hygiene_smart_device_integration() {
    println!("🤖 Starting smart device integration test for hygiene events");

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let (user_id, _api_key) = create_test_user_and_api_key(&pool).await;

    // Test smart device scenarios
    let smart_device_events = vec![
        HygieneIngestRequest {
            recorded_at: Utc::now() - Duration::minutes(30),
            event_type: "handwashing".to_string(),
            duration_seconds: Some(22),
            quality_rating: Some(4),
            meets_who_guidelines: Some(true),
            device_detected: Some(true),
            device_effectiveness_score: Some(92.0),
            trigger_event: Some("automatic_detection".to_string()),
            location_context: Some("work".to_string()),
            compliance_motivation: Some("smart_reminder".to_string()),
            health_crisis_enhanced: Some(false),
            crisis_compliance_level: None,
            daily_goal_progress: Some(75),
            achievement_unlocked: Some("Smart Hygiene Champion".to_string()),
            medication_adherence_related: Some(false),
            medical_condition_context: None,
            data_sensitivity_level: Some("standard".to_string()),
            source_device: Some("oral_b_smart_toothbrush_gen5".to_string()),
            frequency_compliance_rating: Some(4),
        },
        HygieneIngestRequest {
            recorded_at: Utc::now() - Duration::minutes(15),
            event_type: "toothbrushing".to_string(),
            duration_seconds: Some(125),
            quality_rating: Some(5),
            meets_who_guidelines: Some(true),
            device_detected: Some(true),
            device_effectiveness_score: Some(96.0),
            trigger_event: Some("smart_routine".to_string()),
            location_context: Some("home".to_string()),
            compliance_motivation: Some("device_coaching".to_string()),
            health_crisis_enhanced: Some(false),
            crisis_compliance_level: None,
            daily_goal_progress: Some(100),
            achievement_unlocked: None,
            medication_adherence_related: Some(false),
            medical_condition_context: None,
            data_sensitivity_level: Some("standard".to_string()),
            source_device: Some("philips_sonicare_9900_prestige".to_string()),
            frequency_compliance_rating: Some(5),
        },
    ];

    let validation_config = ValidationConfig::default();
    let batch_processor = BatchProcessor::new(pool.clone());
    let auth = AuthContext::new_for_testing(user_id);

    let payload = HygieneIngestPayload {
        hygiene_events: smart_device_events,
    };

    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(batch_processor))
            .app_data(web::Data::new(validation_config))
            .service(web::resource("/ingest/hygiene").route(web::post().to(ingest_hygiene))),
    )
    .await;

    let req = test::TestRequest::post()
        .uri("/ingest/hygiene")
        .insert_header(("content-type", "application/json"))
        .set_json(&payload)
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth);

    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let response: Value = test::read_body_json(resp).await;
    println!(
        "📱 Smart device integration response: {}",
        serde_json::to_string_pretty(&response).unwrap()
    );

    // Verify smart device analysis
    let analysis = response["hygiene_analysis"].as_object().unwrap();

    let smart_device_detections = analysis["smart_device_detections"].as_u64().unwrap();
    assert_eq!(
        smart_device_detections, 2,
        "Both events should be detected by smart devices"
    );

    // Verify stored data includes smart device information
    let stored_events = sqlx::query!(
        r#"SELECT
            event_type::text as "event_type!", device_detected, device_effectiveness_score,
            source_device, achievement_unlocked
        FROM hygiene_events
        WHERE user_id = $1
        ORDER BY recorded_at DESC"#,
        user_id
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to retrieve smart device events");

    assert_eq!(stored_events.len(), 2);

    for event in &stored_events {
        assert_eq!(event.device_detected.unwrap_or(false), true);
        assert!(event.device_effectiveness_score.unwrap_or(0.0) > 90.0);
        assert!(event.source_device.is_some());
    }

    // Verify specific smart device sources
    let device_sources: Vec<String> = stored_events
        .iter()
        .filter_map(|e| e.source_device.as_ref())
        .cloned()
        .collect();

    assert!(device_sources
        .iter()
        .any(|d| d.contains("oral_b") || d.contains("philips")));

    println!("✅ Smart device integration test completed successfully");

    cleanup_test_data(&pool, user_id).await;
}
