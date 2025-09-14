use actix_web::{test, web, App};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use self_sensored::{
    handlers::nutrition_handler,
    middleware::{auth::AuthContext, metrics::Metrics},
    models::health_metrics::NutritionMetric,
    services::{auth::AuthService, rate_limiter::RateLimiter},
};

/// Test nutrition data ingestion endpoint with comprehensive validation
#[actix_web::test]
async fn test_nutrition_ingest_comprehensive() {
    // Skip if test database is not available
    if std::env::var("TEST_DATABASE_URL").is_err() {
        println!("Skipping nutrition integration test - TEST_DATABASE_URL not set");
        return;
    }

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // Set up test user and auth context
    let user_id = Uuid::new_v4();
    let auth_context = AuthContext { user_id };

    // Create comprehensive test nutrition payload with multiple nutrients
    let test_payload = json!({
        "nutrition_metrics": [
            {
                "recorded_at": Utc::now(),
                "dietary_water": 2.5,              // 2.5 liters
                "dietary_caffeine": 150.0,         // 150mg
                "dietary_energy_consumed": 2000.0, // 2000 calories
                "dietary_carbohydrates": 250.0,    // 250g carbs
                "dietary_protein": 75.0,           // 75g protein
                "dietary_fat_total": 65.0,         // 65g fat
                "dietary_fat_saturated": 20.0,     // 20g saturated fat
                "dietary_cholesterol": 200.0,      // 200mg cholesterol
                "dietary_sodium": 2000.0,          // 2000mg sodium
                "dietary_fiber": 25.0,             // 25g fiber
                "dietary_sugar": 50.0,             // 50g sugar
                "dietary_calcium": 1000.0,         // 1000mg calcium
                "dietary_iron": 15.0,              // 15mg iron
                "dietary_magnesium": 300.0,        // 300mg magnesium
                "dietary_potassium": 3000.0,       // 3000mg potassium
                "dietary_vitamin_a": 800.0,        // 800mcg vitamin A
                "dietary_vitamin_c": 90.0,         // 90mg vitamin C
                "dietary_vitamin_d": 600.0,        // 600 IU vitamin D
                "source_device": "Test iPhone"
            },
            {
                "recorded_at": Utc::now(),
                "dietary_water": 0.3,              // 300ml water
                "dietary_energy_consumed": 150.0,  // 150 calories snack
                "dietary_carbohydrates": 20.0,     // 20g carbs
                "dietary_protein": 3.0,            // 3g protein
                "dietary_fat_total": 5.0,          // 5g fat
                "dietary_sugar": 15.0,             // 15g sugar
                "source_device": "Test Apple Watch"
            }
        ]
    });

    // Create test application with necessary services
    let app = test::init_service(
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(Metrics))
            .route(
                "/ingest/nutrition",
                web::post().to(nutrition_handler::ingest_nutrition_data),
            )
            .route(
                "/data/nutrition",
                web::get().to(nutrition_handler::get_nutrition_data),
            )
            .route(
                "/data/hydration",
                web::get().to(nutrition_handler::get_hydration_data),
            ),
    )
    .await;

    // Test nutrition ingestion
    let req = test::TestRequest::post()
        .uri("/ingest/nutrition")
        .insert_header(("content-type", "application/json"))
        .set_json(&test_payload)
        .to_request();

    // Add auth context to request extensions
    let mut req = req;
    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;

    // Verify successful ingestion
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert_eq!(body["success"], true);
    assert_eq!(body["processed_count"], 2);
    assert_eq!(body["failed_count"], 0);

    // Verify nutrition analysis is generated
    assert!(body["nutrition_analysis"].is_object());
    let analysis = &body["nutrition_analysis"];
    assert_eq!(analysis["total_entries"], 2);
    assert!(analysis["hydration_status"].is_object());
    assert!(analysis["macronutrient_analysis"].is_object());

    // Test nutrition data retrieval
    let req = test::TestRequest::get()
        .uri("/data/nutrition?include_analysis=true&daily_aggregation=true")
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth_context.clone());

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["nutrition_data"].is_array());
    assert!(body["summary"].is_object());
    assert!(body["daily_aggregations"].is_array());

    // Test hydration-specific endpoint
    let req = test::TestRequest::get()
        .uri("/data/hydration?include_caffeine=true")
        .to_request();

    let mut req = req;
    req.extensions_mut().insert(auth_context);

    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 200);

    let body: serde_json::Value = test::read_body_json(resp).await;
    assert!(body["hydration_data"].is_array());
    assert!(body["hydration_summary"].is_object());

    // Clean up test data
    let _cleanup = sqlx::query!("DELETE FROM nutrition_metrics WHERE user_id = $1", user_id)
        .execute(&pool)
        .await
        .expect("Failed to clean up test data");

    println!(" Comprehensive nutrition integration test passed!");
}

/// Test nutrition validation with edge cases
#[actix_web::test]
async fn test_nutrition_validation_edge_cases() {
    // Test with invalid nutritional values
    let test_cases = vec![
        // Excessive water intake
        json!({
            "nutrition_metrics": [{
                "recorded_at": Utc::now(),
                "dietary_water": 15.0,  // 15L - excessive
            }]
        }),
        // Negative caffeine
        json!({
            "nutrition_metrics": [{
                "recorded_at": Utc::now(),
                "dietary_caffeine": -50.0,  // Negative
            }]
        }),
        // Excessive calories
        json!({
            "nutrition_metrics": [{
                "recorded_at": Utc::now(),
                "dietary_energy_consumed": 15000.0,  // 15k calories - unrealistic
            }]
        }),
        // Excessive sodium
        json!({
            "nutrition_metrics": [{
                "recorded_at": Utc::now(),
                "dietary_sodium": 15000.0,  // 15g sodium - dangerous
            }]
        })
    ];

    for (i, test_payload) in test_cases.iter().enumerate() {
        println!("Testing validation case {}", i + 1);

        // Create a test metric from the payload and validate directly
        let nutrition_data = &test_payload["nutrition_metrics"][0];
        let test_metric = NutritionMetric {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            dietary_water: nutrition_data.get("dietary_water").and_then(|v| v.as_f64()),
            dietary_caffeine: nutrition_data.get("dietary_caffeine").and_then(|v| v.as_f64()),
            dietary_energy_consumed: nutrition_data.get("dietary_energy_consumed").and_then(|v| v.as_f64()),
            dietary_carbohydrates: nutrition_data.get("dietary_carbohydrates").and_then(|v| v.as_f64()),
            dietary_protein: nutrition_data.get("dietary_protein").and_then(|v| v.as_f64()),
            dietary_fat_total: nutrition_data.get("dietary_fat_total").and_then(|v| v.as_f64()),
            dietary_fat_saturated: nutrition_data.get("dietary_fat_saturated").and_then(|v| v.as_f64()),
            dietary_cholesterol: nutrition_data.get("dietary_cholesterol").and_then(|v| v.as_f64()),
            dietary_sodium: nutrition_data.get("dietary_sodium").and_then(|v| v.as_f64()),
            dietary_fiber: nutrition_data.get("dietary_fiber").and_then(|v| v.as_f64()),
            dietary_sugar: nutrition_data.get("dietary_sugar").and_then(|v| v.as_f64()),
            dietary_calcium: nutrition_data.get("dietary_calcium").and_then(|v| v.as_f64()),
            dietary_iron: nutrition_data.get("dietary_iron").and_then(|v| v.as_f64()),
            dietary_magnesium: nutrition_data.get("dietary_magnesium").and_then(|v| v.as_f64()),
            dietary_potassium: nutrition_data.get("dietary_potassium").and_then(|v| v.as_f64()),
            dietary_vitamin_a: nutrition_data.get("dietary_vitamin_a").and_then(|v| v.as_f64()),
            dietary_vitamin_c: nutrition_data.get("dietary_vitamin_c").and_then(|v| v.as_f64()),
            dietary_vitamin_d: nutrition_data.get("dietary_vitamin_d").and_then(|v| v.as_f64()),
            source_device: nutrition_data.get("source_device").and_then(|v| v.as_str().map(|s| s.to_string())),
            created_at: Utc::now(),
        };

        // Validate the metric - should fail
        let validation_result = test_metric.validate();
        assert!(validation_result.is_err(), "Validation case {} should have failed", i + 1);

        println!(" Validation case {} properly rejected: {}", i + 1, validation_result.unwrap_err());
    }

    println!(" Nutrition validation edge cases test passed!");
}

/// Test nutrition analysis and dietary pattern recognition
#[test]
fn test_nutrition_analysis_patterns() {
    use self_sensored::models::health_metrics::MacronutrientDistribution;

    // Test macronutrient distribution calculation
    let test_metric = NutritionMetric {
        id: Uuid::new_v4(),
        user_id: Uuid::new_v4(),
        recorded_at: Utc::now(),
        dietary_water: Some(2.0),
        dietary_caffeine: Some(100.0),
        dietary_energy_consumed: Some(2000.0),
        dietary_carbohydrates: Some(250.0),  // 250g * 4 = 1000 calories (50%)
        dietary_protein: Some(100.0),        // 100g * 4 = 400 calories (20%)
        dietary_fat_total: Some(67.0),       // 67g * 9 = 603 calories (30%)
        dietary_fat_saturated: None,
        dietary_cholesterol: None,
        dietary_sodium: Some(2000.0),
        dietary_fiber: Some(30.0),
        dietary_sugar: Some(40.0),
        dietary_calcium: Some(1000.0),
        dietary_iron: Some(15.0),
        dietary_magnesium: Some(400.0),
        dietary_potassium: Some(3500.0),
        dietary_vitamin_a: Some(900.0),
        dietary_vitamin_c: Some(90.0),
        dietary_vitamin_d: Some(600.0),
        source_device: Some("Test Device".to_string()),
        created_at: Utc::now(),
    };

    // Test macronutrient distribution calculation
    let distribution = test_metric.macronutrient_distribution().unwrap();

    // Allow some rounding tolerance
    assert!((distribution.carbohydrate_percent as i32 - 50).abs() <= 2,
           "Carbs should be ~50%, got {}", distribution.carbohydrate_percent);
    assert!((distribution.protein_percent as i32 - 20).abs() <= 2,
           "Protein should be ~20%, got {}", distribution.protein_percent);
    assert!((distribution.fat_percent as i32 - 30).abs() <= 2,
           "Fat should be ~30%, got {}", distribution.fat_percent);

    // Test hydration status
    assert_eq!(test_metric.hydration_status(), "adequate");

    // Test excessive sodium detection
    assert!(!test_metric.has_excessive_sodium()); // 2000mg is not excessive

    // Test caffeine limit check
    assert!(!test_metric.exceeds_caffeine_limit()); // 100mg is fine

    // Test balanced meal detection
    assert!(test_metric.is_balanced_meal()); // Should be balanced with 50/20/30 distribution

    println!(" Nutrition analysis and pattern recognition test passed!");
}

/// Integration test for comprehensive nutrition tracking over time
#[actix_web::test]
async fn test_nutrition_tracking_timeline() {
    // Skip if test database is not available
    if std::env::var("TEST_DATABASE_URL").is_err() {
        println!("Skipping nutrition timeline test - TEST_DATABASE_URL not set");
        return;
    }

    let database_url = std::env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set for integration tests");

    let pool = sqlx::PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    let user_id = Uuid::new_v4();

    // Create a week's worth of nutrition data with different patterns
    let mut test_metrics = Vec::new();

    for day in 0..7 {
        let base_time = Utc::now() - chrono::Duration::days(day);

        // Breakfast
        test_metrics.push(NutritionMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time + chrono::Duration::hours(8),
            dietary_energy_consumed: Some(400.0),
            dietary_carbohydrates: Some(60.0),
            dietary_protein: Some(15.0),
            dietary_fat_total: Some(12.0),
            dietary_water: Some(0.5),
            dietary_caffeine: Some(if day % 2 == 0 { 100.0 } else { 0.0 }),
            dietary_fiber: Some(8.0),
            dietary_vitamin_c: Some(30.0),
            source_device: Some("Test App".to_string()),
            ..Default::default()
        });

        // Lunch
        test_metrics.push(NutritionMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time + chrono::Duration::hours(13),
            dietary_energy_consumed: Some(600.0),
            dietary_carbohydrates: Some(75.0),
            dietary_protein: Some(35.0),
            dietary_fat_total: Some(20.0),
            dietary_water: Some(0.8),
            dietary_sodium: Some(800.0),
            dietary_fiber: Some(12.0),
            dietary_iron: Some(8.0),
            source_device: Some("Test App".to_string()),
            ..Default::default()
        });

        // Dinner
        test_metrics.push(NutritionMetric {
            id: Uuid::new_v4(),
            user_id,
            recorded_at: base_time + chrono::Duration::hours(19),
            dietary_energy_consumed: Some(700.0),
            dietary_carbohydrates: Some(85.0),
            dietary_protein: Some(40.0),
            dietary_fat_total: Some(25.0),
            dietary_water: Some(1.2),
            dietary_sodium: Some(600.0),
            dietary_calcium: Some(400.0),
            dietary_potassium: Some(1500.0),
            source_device: Some("Test App".to_string()),
            ..Default::default()
        });
    }

    // Insert all test metrics (simplified direct insertion for test)
    for metric in &test_metrics {
        let _ = sqlx::query!(
            r#"
            INSERT INTO nutrition_metrics (
                id, user_id, recorded_at, dietary_energy_consumed,
                dietary_carbohydrates, dietary_protein, dietary_fat_total,
                dietary_water, dietary_caffeine, dietary_sodium, dietary_fiber,
                dietary_vitamin_c, dietary_iron, dietary_calcium, dietary_potassium,
                source_device, created_at
            ) VALUES (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17
            ) ON CONFLICT (user_id, recorded_at) DO NOTHING
            "#,
            metric.id,
            metric.user_id,
            metric.recorded_at,
            metric.dietary_energy_consumed,
            metric.dietary_carbohydrates,
            metric.dietary_protein,
            metric.dietary_fat_total,
            metric.dietary_water,
            metric.dietary_caffeine,
            metric.dietary_sodium,
            metric.dietary_fiber,
            metric.dietary_vitamin_c,
            metric.dietary_iron,
            metric.dietary_calcium,
            metric.dietary_potassium,
            metric.source_device,
            metric.created_at
        )
        .execute(&pool)
        .await
        .expect("Failed to insert test metric");
    }

    println!(" Inserted {} nutrition metrics for timeline test", test_metrics.len());

    // Verify data was inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM nutrition_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await
    .expect("Failed to count test metrics");

    assert_eq!(count.unwrap_or(0), test_metrics.len() as i64);

    // Clean up
    let _cleanup = sqlx::query!("DELETE FROM nutrition_metrics WHERE user_id = $1", user_id)
        .execute(&pool)
        .await
        .expect("Failed to clean up test data");

    println!(" Nutrition tracking timeline test passed!");
}

// Add Default implementation for NutritionMetric for testing
impl Default for NutritionMetric {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            user_id: Uuid::new_v4(),
            recorded_at: Utc::now(),
            dietary_water: None,
            dietary_caffeine: None,
            dietary_energy_consumed: None,
            dietary_carbohydrates: None,
            dietary_protein: None,
            dietary_fat_total: None,
            dietary_fat_saturated: None,
            dietary_cholesterol: None,
            dietary_sodium: None,
            dietary_fiber: None,
            dietary_sugar: None,
            dietary_calcium: None,
            dietary_iron: None,
            dietary_magnesium: None,
            dietary_potassium: None,
            dietary_vitamin_a: None,
            dietary_vitamin_c: None,
            dietary_vitamin_d: None,
            source_device: None,
            created_at: Utc::now(),
        }
    }
}