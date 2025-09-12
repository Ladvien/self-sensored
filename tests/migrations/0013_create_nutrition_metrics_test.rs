use chrono::{DateTime, Utc, TimeZone};
use sqlx::PgPool;
use std::time::Instant;
use uuid::Uuid;

#[sqlx::test]
async fn test_create_nutrition_metrics_table(pool: PgPool) -> sqlx::Result<()> {
    // Verify table was created with correct structure
    let result = sqlx::query!(
        "SELECT column_name, data_type, is_nullable, column_default 
         FROM information_schema.columns 
         WHERE table_name = 'nutrition_metrics' 
         ORDER BY ordinal_position"
    )
    .fetch_all(&pool)
    .await?;

    assert!(!result.is_empty(), "nutrition_metrics table should exist");

    // Verify all essential columns exist (37 nutrition fields + metadata)
    let column_names: Vec<&str> = result.iter().map(|r| r.column_name.as_str()).collect();
    let expected_columns = [
        // Core fields
        "id", "user_id", "recorded_at",
        // Hydration
        "water_ml",
        // Macronutrients
        "energy_consumed_kcal", "carbohydrates_g", "protein_g", "fat_total_g",
        "fat_saturated_g", "fat_monounsaturated_g", "fat_polyunsaturated_g",
        "cholesterol_mg", "fiber_g", "sugar_g", "sodium_mg",
        // Vitamins
        "vitamin_a_mcg", "vitamin_d_mcg", "vitamin_e_mg", "vitamin_k_mcg",
        "vitamin_c_mg", "thiamin_mg", "riboflavin_mg", "niacin_mg",
        "pantothenic_acid_mg", "vitamin_b6_mg", "biotin_mcg", "folate_mcg", "vitamin_b12_mcg",
        // Minerals
        "calcium_mg", "phosphorus_mg", "magnesium_mg", "potassium_mg", "chloride_mg",
        "iron_mg", "zinc_mg", "copper_mg", "manganese_mg", "iodine_mcg",
        "selenium_mcg", "chromium_mcg", "molybdenum_mcg",
        // Other
        "caffeine_mg",
        // Metadata
        "source", "raw_data", "aggregation_period", "created_at"
    ];

    for expected_col in &expected_columns {
        assert!(
            column_names.contains(expected_col),
            "Column '{}' should exist in nutrition_metrics", 
            expected_col
        );
    }

    // Verify we have 37+ fields total (35+ nutrition fields plus metadata)
    assert!(
        column_names.len() >= 41, 
        "Should have at least 41 columns, found: {}", 
        column_names.len()
    );

    Ok(())
}

#[sqlx::test]
async fn test_partitioning_setup(pool: PgPool) -> sqlx::Result<()> {
    // Verify table is partitioned
    let is_partitioned = sqlx::query_scalar!(
        "SELECT COUNT(*) > 0 FROM pg_partitioned_table WHERE partrelid = 'nutrition_metrics'::regclass"
    )
    .fetch_one(&pool)
    .await?;

    assert!(is_partitioned.unwrap_or(false), "nutrition_metrics should be partitioned");

    // Verify initial partitions were created (should have 3+ months)
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'nutrition_metrics_%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 3, 
        "Should have at least 3 partitions, found: {:?}", 
        partition_count
    );

    Ok(())
}

#[sqlx::test]
async fn test_brin_indexes_created(pool: PgPool) -> sqlx::Result<()> {
    // Verify BRIN indexes were created
    let brin_indexes = sqlx::query!(
        "SELECT indexname FROM pg_indexes 
         WHERE tablename = 'nutrition_metrics' 
         AND indexname LIKE '%_brin'"
    )
    .fetch_all(&pool)
    .await?;

    assert!(
        brin_indexes.len() >= 3,
        "Should have at least 3 BRIN indexes, found: {}",
        brin_indexes.len()
    );

    // Verify specific BRIN indexes exist
    let index_names: Vec<&str> = brin_indexes.iter().map(|r| r.indexname.as_str()).collect();
    assert!(index_names.iter().any(|&name| name.contains("recorded_at")));
    assert!(index_names.iter().any(|&name| name.contains("user_recorded")));

    Ok(())
}

#[sqlx::test]
async fn test_insert_basic_nutrition_data(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "test@nutrition.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(12, 0, 0);

    // Insert basic nutrition data with key macronutrients
    sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at, water_ml, energy_consumed_kcal, 
            carbohydrates_g, protein_g, fat_total_g, fiber_g, sodium_mg, 
            vitamin_c_mg, calcium_mg, iron_mg, caffeine_mg, source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)",
        user_id,
        recorded_at,
        2000.0, // 2L water
        2200.5, // 2200 kcal
        275.0,  // 275g carbs
        120.0,  // 120g protein
        85.0,   // 85g fat
        35.0,   // 35g fiber
        2300.0, // 2300mg sodium
        90.0,   // 90mg vitamin C
        1200.0, // 1200mg calcium
        15.5,   // 15.5mg iron
        200.0,  // 200mg caffeine
        "MyFitnessPal"
    )
    .execute(&pool)
    .await?;

    // Verify insertion
    let result = sqlx::query!(
        "SELECT user_id, water_ml, energy_consumed_kcal, carbohydrates_g, 
                protein_g, fat_total_g, vitamin_c_mg, calcium_mg, iron_mg, caffeine_mg
         FROM nutrition_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.user_id, user_id);
    assert_eq!(result.water_ml, Some(2000.0));
    assert_eq!(result.energy_consumed_kcal, Some(2200.5));
    assert_eq!(result.carbohydrates_g, Some(275.0));
    assert_eq!(result.protein_g, Some(120.0));
    assert_eq!(result.fat_total_g, Some(85.0));
    assert_eq!(result.vitamin_c_mg, Some(90.0));
    assert_eq!(result.calcium_mg, Some(1200.0));
    assert_eq!(result.iron_mg, Some(15.5));
    assert_eq!(result.caffeine_mg, Some(200.0));

    Ok(())
}

#[sqlx::test]
async fn test_comprehensive_vitamin_insert(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "vitamins@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(13, 0, 0);

    // Insert comprehensive vitamin data
    sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at,
            vitamin_a_mcg, vitamin_c_mg, vitamin_d_mcg, vitamin_e_mg, vitamin_k_mcg,
            thiamin_mg, riboflavin_mg, niacin_mg, pantothenic_acid_mg, 
            vitamin_b6_mg, biotin_mcg, folate_mcg, vitamin_b12_mcg, source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
        user_id,
        recorded_at,
        900.0,  // Vitamin A
        85.0,   // Vitamin C
        25.0,   // Vitamin D
        15.0,   // Vitamin E
        120.0,  // Vitamin K
        1.2,    // Thiamin (B1)
        1.3,    // Riboflavin (B2)
        16.0,   // Niacin (B3)
        5.0,    // Pantothenic acid (B5)
        1.7,    // Vitamin B6
        30.0,   // Biotin (B7)
        400.0,  // Folate (B9)
        2.4,    // Vitamin B12
        "Cronometer"
    )
    .execute(&pool)
    .await?;

    // Verify vitamin data
    let result = sqlx::query!(
        "SELECT vitamin_a_mcg, vitamin_c_mg, vitamin_d_mcg, thiamin_mg, 
                riboflavin_mg, vitamin_b6_mg, vitamin_b12_mcg
         FROM nutrition_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.vitamin_a_mcg, Some(900.0));
    assert_eq!(result.vitamin_c_mg, Some(85.0));
    assert_eq!(result.vitamin_d_mcg, Some(25.0));
    assert_eq!(result.thiamin_mg, Some(1.2));
    assert_eq!(result.riboflavin_mg, Some(1.3));
    assert_eq!(result.vitamin_b6_mg, Some(1.7));
    assert_eq!(result.vitamin_b12_mcg, Some(2.4));

    Ok(())
}

#[sqlx::test]
async fn test_comprehensive_mineral_insert(pool: PgPool) -> sqlx::Result<()> {
    // Create test user
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "minerals@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(14, 0, 0);

    // Insert comprehensive mineral data
    sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at,
            calcium_mg, phosphorus_mg, magnesium_mg, potassium_mg, sodium_mg, chloride_mg,
            iron_mg, zinc_mg, copper_mg, manganese_mg, 
            iodine_mcg, selenium_mcg, chromium_mcg, molybdenum_mcg, source
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)",
        user_id,
        recorded_at,
        1000.0,  // Calcium
        700.0,   // Phosphorus
        400.0,   // Magnesium
        3500.0,  // Potassium
        2300.0,  // Sodium
        3400.0,  // Chloride
        18.0,    // Iron
        11.0,    // Zinc
        0.9,     // Copper
        2.3,     // Manganese
        150.0,   // Iodine
        55.0,    // Selenium
        35.0,    // Chromium
        45.0,    // Molybdenum
        "NutritionData"
    )
    .execute(&pool)
    .await?;

    // Verify mineral data
    let result = sqlx::query!(
        "SELECT calcium_mg, iron_mg, zinc_mg, magnesium_mg, potassium_mg, 
                iodine_mcg, selenium_mcg
         FROM nutrition_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.calcium_mg, Some(1000.0));
    assert_eq!(result.iron_mg, Some(18.0));
    assert_eq!(result.zinc_mg, Some(11.0));
    assert_eq!(result.magnesium_mg, Some(400.0));
    assert_eq!(result.potassium_mg, Some(3500.0));
    assert_eq!(result.iodine_mcg, Some(150.0));
    assert_eq!(result.selenium_mcg, Some(55.0));

    Ok(())
}

#[sqlx::test]
async fn test_validation_constraints_hydration(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "hydration@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(15, 0, 0);

    // Test valid water intake
    let valid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (user_id, recorded_at, water_ml) VALUES ($1, $2, $3)",
        user_id,
        recorded_at,
        5000.0 // 5L - reasonable maximum
    )
    .execute(&pool)
    .await;

    assert!(valid_insert.is_ok(), "Valid water intake should succeed");

    // Test invalid water intake (too high)
    let recorded_at_invalid = Utc.ymd(2025, 9, 11).and_hms(16, 0, 0);
    let invalid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (user_id, recorded_at, water_ml) VALUES ($1, $2, $3)",
        user_id,
        recorded_at_invalid,
        25000.0 // 25L - too high
    )
    .execute(&pool)
    .await;

    assert!(invalid_insert.is_err(), "Invalid water intake should fail");

    Ok(())
}

#[sqlx::test]
async fn test_validation_constraints_macronutrients(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "macros@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(17, 0, 0);

    // Test valid macronutrient values
    let valid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at, energy_consumed_kcal, carbohydrates_g, 
            protein_g, fat_total_g, fiber_g
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        user_id,
        recorded_at,
        3500.0, // High but reasonable for athletes
        400.0,  // High carbs
        200.0,  // High protein
        150.0,  // High fat
        50.0    // High fiber
    )
    .execute(&pool)
    .await;

    assert!(valid_insert.is_ok(), "Valid macronutrient values should succeed");

    // Test invalid protein (too high)
    let recorded_at_invalid = Utc.ymd(2025, 9, 11).and_hms(18, 0, 0);
    let invalid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (user_id, recorded_at, protein_g) VALUES ($1, $2, $3)",
        user_id,
        recorded_at_invalid,
        1500.0 // Extremely high protein
    )
    .execute(&pool)
    .await;

    assert!(invalid_insert.is_err(), "Invalid protein amount should fail");

    Ok(())
}

#[sqlx::test]
async fn test_validation_constraints_vitamins(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "vitcheck@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(19, 0, 0);

    // Test valid vitamin values (high but within safety limits)
    let valid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at, vitamin_c_mg, vitamin_d_mcg, vitamin_a_mcg
        ) VALUES ($1, $2, $3, $4, $5)",
        user_id,
        recorded_at,
        1000.0, // High vitamin C
        100.0,  // High vitamin D
        3000.0  // High vitamin A
    )
    .execute(&pool)
    .await;

    assert!(valid_insert.is_ok(), "Valid vitamin values should succeed");

    // Test comprehensive vitamin upper limit boundary cases
    let vitamin_boundary_tests = [
        // Vitamin A - Upper limit around 3000 mcg (10000 IU)
        ("vitamin_a_mcg", 3000.0, true),   // At safe upper limit
        ("vitamin_a_mcg", 3001.0, false), // Just over safe limit
        ("vitamin_a_mcg", 5000.0, false), // Dangerous level
        ("vitamin_a_mcg", 10000.0, false), // Extremely dangerous
        
        // Vitamin D - Upper limit around 100 mcg (4000 IU)
        ("vitamin_d_mcg", 100.0, true),   // At safe upper limit
        ("vitamin_d_mcg", 125.0, false),  // Over safe limit
        ("vitamin_d_mcg", 250.0, false),  // Dangerous level
        ("vitamin_d_mcg", 1000.0, false), // Extremely dangerous
        
        // Vitamin E - Upper limit around 1000 mg
        ("vitamin_e_mg", 1000.0, true),   // At safe upper limit
        ("vitamin_e_mg", 1200.0, false),  // Over safe limit
        ("vitamin_e_mg", 2000.0, false),  // Dangerous level
        
        // Vitamin C - Upper limit around 2000 mg for most people
        ("vitamin_c_mg", 2000.0, true),   // At tolerable upper limit
        ("vitamin_c_mg", 3000.0, false),  // Over upper limit
        ("vitamin_c_mg", 6000.0, false),  // Dangerously high
        ("vitamin_c_mg", 10000.0, false), // Extremely dangerous
        
        // Vitamin K - Generally safe but test reasonable upper bounds
        ("vitamin_k_mcg", 1000.0, true),  // High but safe
        ("vitamin_k_mcg", 5000.0, false), // Very high
        
        // B Vitamins - Test upper limits
        ("vitamin_b6_mg", 100.0, true),   // Upper safe limit
        ("vitamin_b6_mg", 200.0, false),  // Over safe limit (can cause neuropathy)
        ("vitamin_b6_mg", 500.0, false),  // Dangerous
        
        // Niacin (B3) - Upper limit around 35 mg from supplements
        ("niacin_mg", 35.0, true),        // Safe upper limit
        ("niacin_mg", 100.0, false),      // Can cause flushing
        ("niacin_mg", 500.0, false),      // Liver toxicity risk
        
        // Folate - Upper limit 1000 mcg from supplements
        ("folate_mcg", 1000.0, true),     // Safe upper limit
        ("folate_mcg", 1500.0, false),    // Over safe limit
        ("folate_mcg", 5000.0, false),    // Dangerous
        
        // B12 - Generally safe even at high doses, but test reasonable bounds
        ("vitamin_b12_mcg", 1000.0, true), // High therapeutic dose
        ("vitamin_b12_mcg", 5000.0, true), // Very high but still safe
        ("vitamin_b12_mcg", 50000.0, false), // Unreasonably high
    ];

    for (i, (field_name, value, should_succeed)) in vitamin_boundary_tests.iter().enumerate() {
        let test_time = recorded_at_invalid + chrono::Duration::minutes(i as i64 + 1);
        
        let insert_query = match *field_name {
            "vitamin_a_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_a_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "vitamin_d_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_d_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "vitamin_e_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_e_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "vitamin_c_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_c_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "vitamin_k_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_k_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "vitamin_b6_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_b6_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "niacin_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, niacin_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "folate_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, folate_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "vitamin_b12_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, vitamin_b12_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            _ => panic!("Unknown vitamin field: {}", field_name)
        };

        if *should_succeed {
            assert!(
                insert_query.is_ok(),
                "Vitamin {} at {} should succeed (within safe limits)",
                field_name, value
            );
        } else {
            assert!(
                insert_query.is_err(),
                "Vitamin {} at {} should fail (exceeds safe limits)",
                field_name, value
            );
        }
    }

    Ok(())
}

#[sqlx::test]
async fn test_validation_constraints_minerals(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "mincheck@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(21, 0, 0);

    // Test valid mineral values
    let valid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at, calcium_mg, iron_mg, zinc_mg, sodium_mg
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        recorded_at,
        2000.0, // High calcium
        25.0,   // High iron
        15.0,   // High zinc
        4000.0  // High sodium
    )
    .execute(&pool)
    .await;

    assert!(valid_insert.is_ok(), "Valid mineral values should succeed");

    // Test comprehensive mineral upper limit boundary cases
    let mineral_boundary_tests = [
        // Iron - Upper limit around 45 mg for adults
        ("iron_mg", 45.0, true),      // Safe upper limit
        ("iron_mg", 65.0, false),     // Over safe limit
        ("iron_mg", 100.0, false),    // Dangerous
        ("iron_mg", 300.0, false),    // Extremely dangerous
        
        // Zinc - Upper limit around 40 mg
        ("zinc_mg", 40.0, true),      // Safe upper limit
        ("zinc_mg", 60.0, false),     // Over safe limit (can interfere with copper)
        ("zinc_mg", 100.0, false),    // Dangerous
        
        // Copper - Upper limit around 10 mg
        ("copper_mg", 10.0, true),    // Safe upper limit
        ("copper_mg", 15.0, false),   // Over safe limit
        ("copper_mg", 50.0, false),   // Dangerous
        
        // Selenium - Upper limit around 400 mcg
        ("selenium_mcg", 400.0, true), // Safe upper limit
        ("selenium_mcg", 600.0, false), // Over safe limit (selenium toxicity)
        ("selenium_mcg", 1000.0, false), // Dangerous
        
        // Manganese - Upper limit around 11 mg
        ("manganese_mg", 11.0, true), // Safe upper limit
        ("manganese_mg", 20.0, false), // Over safe limit
        ("manganese_mg", 50.0, false), // Dangerous
        
        // Iodine - Upper limit around 1100 mcg
        ("iodine_mcg", 1100.0, true), // Safe upper limit
        ("iodine_mcg", 1500.0, false), // Over safe limit
        ("iodine_mcg", 3000.0, false), // Dangerous
        
        // Chromium - Upper limit around 200-300 mcg
        ("chromium_mcg", 200.0, true), // Safe level
        ("chromium_mcg", 1000.0, false), // Very high
        ("chromium_mcg", 5000.0, false), // Extremely high
        
        // Molybdenum - Upper limit around 2000 mcg
        ("molybdenum_mcg", 2000.0, true), // Safe upper limit
        ("molybdenum_mcg", 3000.0, false), // Over safe limit
        ("molybdenum_mcg", 10000.0, false), // Dangerous
        
        // Calcium - Generally safe but very high amounts can cause issues
        ("calcium_mg", 2500.0, true),  // High but tolerable
        ("calcium_mg", 4000.0, false), // Over upper limit (kidney stones risk)
        ("calcium_mg", 10000.0, false), // Dangerous
        
        // Magnesium - Upper limit for supplements around 350 mg
        ("magnesium_mg", 350.0, true), // Safe supplemental limit
        ("magnesium_mg", 700.0, true),  // High from food (generally safe)
        ("magnesium_mg", 5000.0, false), // Unreasonably high
        
        // Potassium - Very high amounts could be dangerous
        ("potassium_mg", 4700.0, true), // Adequate intake level
        ("potassium_mg", 10000.0, true), // High but generally safe from food
        ("potassium_mg", 50000.0, false), // Dangerously high
        
        // Sodium - Upper limits for health
        ("sodium_mg", 2300.0, true),    // Recommended limit
        ("sodium_mg", 6000.0, true),    // High but possible from food
        ("sodium_mg", 20000.0, false),  // Extremely high and dangerous
        
        // Phosphorus - Upper limit around 4000 mg
        ("phosphorus_mg", 4000.0, true), // Safe upper limit
        ("phosphorus_mg", 6000.0, false), // Over safe limit
        ("phosphorus_mg", 15000.0, false), // Dangerous
    ];

    let recorded_at_invalid = Utc.ymd(2025, 9, 11).and_hms(22, 0, 0);
    
    for (i, (field_name, value, should_succeed)) in mineral_boundary_tests.iter().enumerate() {
        let test_time = recorded_at_invalid + chrono::Duration::minutes(i as i64 + 1);
        
        let insert_query = match *field_name {
            "iron_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, iron_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "zinc_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, zinc_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "copper_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, copper_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "selenium_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, selenium_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "manganese_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, manganese_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "iodine_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, iodine_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "chromium_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, chromium_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "molybdenum_mcg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, molybdenum_mcg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "calcium_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, calcium_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "magnesium_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, magnesium_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "potassium_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, potassium_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "sodium_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, sodium_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            "phosphorus_mg" => {
                sqlx::query!(
                    "INSERT INTO nutrition_metrics (user_id, recorded_at, phosphorus_mg) VALUES ($1, $2, $3)",
                    user_id, test_time, value
                ).execute(&pool).await
            },
            _ => panic!("Unknown mineral field: {}", field_name)
        };

        if *should_succeed {
            assert!(
                insert_query.is_ok(),
                "Mineral {} at {} should succeed (within safe limits)",
                field_name, value
            );
        } else {
            assert!(
                insert_query.is_err(),
                "Mineral {} at {} should fail (exceeds safe limits)",
                field_name, value
            );
        }
    }

    Ok(())
}

#[sqlx::test]
async fn test_decimal_precision_handling(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "precision@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(23, 0, 0);

    // Test high precision values
    sqlx::query!(
        "INSERT INTO nutrition_metrics (
            user_id, recorded_at, water_ml, vitamin_b6_mg, iron_mg, thiamin_mg
        ) VALUES ($1, $2, $3, $4, $5, $6)",
        user_id,
        recorded_at,
        2456.789, // High precision water
        1.567,    // High precision B6
        12.345,   // High precision iron  
        0.987     // High precision thiamin
    )
    .execute(&pool)
    .await?;

    let result = sqlx::query!(
        "SELECT water_ml, vitamin_b6_mg, iron_mg, thiamin_mg
         FROM nutrition_metrics 
         WHERE user_id = $1 AND recorded_at = $2",
        user_id,
        recorded_at
    )
    .fetch_one(&pool)
    .await?;

    // Verify precision is maintained (NUMERIC(8,2) for most, NUMERIC(8,3) for trace)
    assert_eq!(result.water_ml, Some(2456.79)); // Rounded to 2 decimals
    assert_eq!(result.vitamin_b6_mg, Some(1.567)); // 3 decimals for trace vitamins
    assert_eq!(result.iron_mg, Some(12.345)); // 3 decimals for trace minerals
    assert_eq!(result.thiamin_mg, Some(0.987)); // 3 decimals for trace vitamins

    Ok(())
}

#[sqlx::test]
async fn test_aggregation_period_enum(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "aggregation@test.com"
    )
    .execute(&pool)
    .await?;

    let recorded_at = Utc.ymd(2025, 9, 11).and_hms(12, 30, 0);

    // Test valid aggregation periods
    let valid_periods = ["meal", "daily", "weekly"];
    for (i, period) in valid_periods.iter().enumerate() {
        let test_time = recorded_at + chrono::Duration::hours(i as i64);
        let insert_result = sqlx::query!(
            "INSERT INTO nutrition_metrics (user_id, recorded_at, aggregation_period, energy_consumed_kcal) 
             VALUES ($1, $2, $3, $4)",
            user_id,
            test_time,
            period,
            500.0
        )
        .execute(&pool)
        .await;

        assert!(insert_result.is_ok(), "Valid aggregation period '{}' should succeed", period);
    }

    // Test invalid aggregation period
    let invalid_time = recorded_at + chrono::Duration::hours(10);
    let invalid_insert = sqlx::query!(
        "INSERT INTO nutrition_metrics (user_id, recorded_at, aggregation_period, energy_consumed_kcal) 
         VALUES ($1, $2, $3, $4)",
        user_id,
        invalid_time,
        "invalid_period",
        500.0
    )
    .execute(&pool)
    .await;

    assert!(invalid_insert.is_err(), "Invalid aggregation period should fail");

    Ok(())
}

#[sqlx::test]
async fn test_nutrition_daily_summary_view(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "summary@test.com"
    )
    .execute(&pool)
    .await?;

    let base_date = Utc.ymd(2025, 9, 11);
    
    // Insert multiple meal entries for the same day
    let meals = [
        (base_date.and_hms(8, 0, 0), 600.0, 75.0, 20.0, 15.0), // Breakfast
        (base_date.and_hms(12, 0, 0), 800.0, 90.0, 35.0, 25.0), // Lunch
        (base_date.and_hms(18, 0, 0), 700.0, 80.0, 30.0, 20.0), // Dinner
    ];

    for (meal_time, calories, carbs, protein, fat) in &meals {
        sqlx::query!(
            "INSERT INTO nutrition_metrics (
                user_id, recorded_at, aggregation_period, energy_consumed_kcal,
                carbohydrates_g, protein_g, fat_total_g, water_ml, calcium_mg
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            user_id,
            meal_time,
            "meal",
            calories,
            carbs,
            protein,
            fat,
            500.0, // 500ml water each meal
            200.0  // 200mg calcium each meal
        )
        .execute(&pool)
        .await?;
    }

    // Query the daily summary view
    let summary = sqlx::query!(
        "SELECT total_energy_kcal, total_carbs_g, total_protein_g, total_fat_g, 
                total_water_ml, total_calcium_mg, total_entries
         FROM nutrition_metrics_daily_summary 
         WHERE user_id = $1 AND nutrition_date = $2",
        user_id,
        base_date.naive_utc().date()
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(summary.total_energy_kcal, Some(2100.0)); // 600+800+700
    assert_eq!(summary.total_carbs_g, Some(245.0)); // 75+90+80  
    assert_eq!(summary.total_protein_g, Some(85.0)); // 20+35+30
    assert_eq!(summary.total_fat_g, Some(60.0)); // 15+25+20
    assert_eq!(summary.total_water_ml, Some(1500.0)); // 500*3
    assert_eq!(summary.total_calcium_mg, Some(600.0)); // 200*3
    assert_eq!(summary.total_entries, Some(3)); // 3 meals

    Ok(())
}

#[sqlx::test]
async fn test_performance_benchmark_insert(pool: PgPool) -> sqlx::Result<()> {
    let user_id = Uuid::new_v4();
    sqlx::query!(
        "INSERT INTO users (id, email, created_at, updated_at) VALUES ($1, $2, NOW(), NOW())",
        user_id,
        "perf@test.com"
    )
    .execute(&pool)
    .await?;

    let start_time = Instant::now();
    let batch_size = 100;

    // Insert batch of comprehensive nutrition records
    for i in 0..batch_size {
        let recorded_at = Utc.ymd(2025, 9, 11).and_hms(0, 0, 0) + chrono::Duration::minutes(i * 15);
        
        sqlx::query!(
            "INSERT INTO nutrition_metrics (
                user_id, recorded_at, water_ml, energy_consumed_kcal, 
                carbohydrates_g, protein_g, fat_total_g, fiber_g, sodium_mg,
                vitamin_c_mg, vitamin_d_mcg, calcium_mg, iron_mg, caffeine_mg,
                aggregation_period, source
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)",
            user_id,
            recorded_at,
            250.0 + (i as f64), // Varying water
            500.0 + (i as f64 * 10.0), // Varying calories
            60.0 + (i as f64), // Varying carbs
            25.0 + (i as f64 * 0.5), // Varying protein
            20.0 + (i as f64 * 0.3), // Varying fat
            8.0 + (i as f64 * 0.1), // Varying fiber
            400.0 + (i as f64 * 10.0), // Varying sodium
            15.0 + (i as f64 * 0.5), // Varying vitamin C
            5.0 + (i as f64 * 0.2), // Varying vitamin D
            150.0 + (i as f64 * 5.0), // Varying calcium
            3.0 + (i as f64 * 0.1), // Varying iron
            50.0 + (i as f64), // Varying caffeine
            "meal",
            "Performance Test"
        )
        .execute(&pool)
        .await?;
    }

    let duration = start_time.elapsed();
    
    // Performance benchmark: should complete 100 inserts in under 5 seconds
    assert!(
        duration.as_secs() < 5,
        "Batch insert of {} nutrition records took too long: {:?}",
        batch_size,
        duration
    );

    println!("✅ Inserted {} nutrition records in {:?}", batch_size, duration);

    // Verify all records were inserted
    let count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM nutrition_metrics WHERE user_id = $1",
        user_id
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(count, Some(batch_size as i64));

    Ok(())
}

#[sqlx::test]
async fn test_partition_management_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the partition creation function
    sqlx::query!("SELECT create_nutrition_monthly_partitions(2, 6)")
        .execute(&pool)
        .await?;

    // Verify additional partitions were created
    let partition_count = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM pg_class WHERE relname LIKE 'nutrition_metrics_%'"
    )
    .fetch_one(&pool)
    .await?;

    assert!(
        partition_count.unwrap_or(0) >= 8, // Should have at least 8 partitions now (2 back + 6 ahead)
        "Should have created additional partitions"
    );

    Ok(())
}

#[sqlx::test] 
async fn test_performance_monitoring_function(pool: PgPool) -> sqlx::Result<()> {
    // Test the performance analysis function
    let result = sqlx::query!(
        "SELECT * FROM analyze_nutrition_performance()"
    )
    .fetch_one(&pool)
    .await?;

    assert_eq!(result.table_name, Some("nutrition_metrics".to_string()));
    assert_eq!(result.total_fields_with_data, Some(37)); // Should report 37 fields
    assert!(result.partition_count.unwrap_or(0) > 0, "Should have partitions");

    Ok(())
}