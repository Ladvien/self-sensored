use sqlx::{PgPool, Row};
use std::collections::HashSet;
use chrono::{DateTime, Utc};

#[cfg(test)]
mod tests {
    use super::*;
    use self_sensored::db::database::create_connection_pool;
    use std::env;

    async fn create_test_pool() -> PgPool {
        let database_url = env::var("TEST_DATABASE_URL")
            .or_else(|_| env::var("DATABASE_URL"))
            .expect("TEST_DATABASE_URL or DATABASE_URL must be set for integration tests");

        create_connection_pool(&database_url)
            .await
            .expect("Failed to create test database connection pool")
    }

    #[tokio::test]
    async fn test_all_required_tables_exist() {
        let pool = create_test_pool().await;
        let expected_tables = vec![
            "users", "api_keys", "audit_log", "raw_ingestions", "heart_rate_metrics",
            "blood_pressure_metrics", "sleep_metrics", "activity_metrics", "workouts",
            "workout_routes", "processing_status", "rate_limit_tracking", 
            "data_quality_metrics", "user_preferences", "api_usage_stats",
            // Partitioned tables
            "raw_ingestions_partitioned", "audit_log_partitioned", 
            "heart_rate_metrics_partitioned", "blood_pressure_metrics_partitioned",
            "sleep_metrics_partitioned", "activity_metrics_partitioned"
        ];

        let query = r#"
            SELECT table_name 
            FROM information_schema.tables 
            WHERE table_schema = 'public' 
            AND table_type = 'BASE TABLE'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        let actual_tables: HashSet<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("table_name"))
            .collect();

        for expected_table in expected_tables {
            assert!(
                actual_tables.contains(expected_table),
                "Required table '{}' does not exist",
                expected_table
            );
        }
    }

    #[tokio::test]
    async fn test_required_extensions_enabled() {
        let pool = create_test_pool().await;
        let query = r#"
            SELECT extname 
            FROM pg_extension 
            WHERE extname IN ('uuid-ossp', 'postgis')
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        let extensions: HashSet<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("extname"))
            .collect();

        assert!(extensions.contains("uuid-ossp"), "uuid-ossp extension not enabled");
        assert!(extensions.contains("postgis"), "PostGIS extension not enabled");
    }

    #[tokio::test]
    async fn test_partitioned_tables_structure() {
        let pool = create_test_pool().await;
        // Test that partitioned tables exist and are properly configured
        let query = r#"
            SELECT 
                schemaname,
                tablename,
                partitioned_by
            FROM pg_partitioned_table 
            JOIN pg_class ON pg_partitioned_table.partrelid = pg_class.oid
            JOIN pg_namespace ON pg_class.relnamespace = pg_namespace.oid
            WHERE nspname = 'public'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        let partitioned_tables: HashSet<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("tablename"))
            .collect();

        let expected_partitioned = vec![
            "raw_ingestions_partitioned",
            "audit_log_partitioned", 
            "heart_rate_metrics_partitioned",
            "blood_pressure_metrics_partitioned",
            "sleep_metrics_partitioned",
            "activity_metrics_partitioned"
        ];

        for table in expected_partitioned {
            assert!(
                partitioned_tables.contains(table),
                "Table '{}' is not properly partitioned",
                table
            );
        }
    }

    #[tokio::test]
    async fn test_partition_functions_exist() {
        let pool = create_test_pool().await;
        let expected_functions = vec![
            "create_monthly_partitions",
            "maintain_partitions", 
            "drop_old_partitions",
            "refresh_brin_indexes",
            "create_partition_indexes"
        ];

        for function_name in expected_functions {
            let query = r#"
                SELECT COUNT(*) as count
                FROM pg_proc 
                WHERE proname = $1
            "#;
            
            let row = sqlx::query(query)
                .bind(function_name)
                .fetch_one(&pool)
                .await
                .unwrap();
                
            let count: i64 = row.get("count");
            assert!(count > 0, "Function '{}' does not exist", function_name);
        }
    }

    #[tokio::test]
    async fn test_brin_indexes_exist() {
        let pool = create_test_pool().await;
        let query = r#"
            SELECT indexname
            FROM pg_indexes 
            WHERE indexdef LIKE '%USING brin%'
            AND schemaname = 'public'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        let brin_indexes: Vec<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("indexname"))
            .collect();

        // Should have BRIN indexes for time-series data
        assert!(!brin_indexes.is_empty(), "No BRIN indexes found");
        
        // Check for specific BRIN indexes
        let expected_brin_patterns = vec![
            "time_brin", "recorded_at_brin", "created_at_brin", "received_at_brin"
        ];
        
        let has_time_series_brin = brin_indexes.iter().any(|index_name| {
            expected_brin_patterns.iter().any(|pattern| index_name.contains(pattern))
        });
        
        assert!(has_time_series_brin, "No time-series BRIN indexes found");
    }

    #[tokio::test]
    async fn test_spatial_indexes_exist() {
        let pool = create_test_pool().await;
        let query = r#"
            SELECT indexname, indexdef
            FROM pg_indexes 
            WHERE indexdef LIKE '%USING gist%'
            AND tablename IN ('workout_routes', 'workouts')
            AND schemaname = 'public'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        assert!(!rows.is_empty(), "No spatial (GIST) indexes found for GPS data");
    }

    #[tokio::test]
    async fn test_api_keys_dual_format_support() {
        let pool = create_test_pool().await;
        // Test that api_keys table supports both UUID and hashed key formats
        let query = r#"
            SELECT column_name, data_type, is_nullable
            FROM information_schema.columns
            WHERE table_name = 'api_keys' 
            AND column_name = 'key_type'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 1, "api_keys table missing key_type column for dual format support");

        // Test constraint exists for key_type values
        let constraint_query = r#"
            SELECT constraint_name
            FROM information_schema.check_constraints
            WHERE constraint_name LIKE '%key_type%'
        "#;
        
        let constraint_rows = sqlx::query(constraint_query).fetch_all(&pool).await.unwrap();
        assert!(!constraint_rows.is_empty(), "Missing key_type constraint");
    }

    #[tokio::test]
    async fn test_heart_rate_metrics_architecture_compliance() {
        let pool = create_test_pool().await;
        // Test that heart_rate_metrics matches ARCHITECTURE.md specification
        let query = r#"
            SELECT column_name, data_type
            FROM information_schema.columns
            WHERE table_name = 'heart_rate_metrics_partitioned'
            ORDER BY column_name
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        let columns: HashSet<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("column_name"))
            .collect();

        // Check for required columns per ARCHITECTURE.md
        let required_columns = vec![
            "id", "user_id", "recorded_at", "min_bpm", "avg_bpm", "max_bpm", 
            "source", "raw_data", "created_at"
        ];

        for col in required_columns {
            assert!(columns.contains(col), "heart_rate_metrics missing column: {}", col);
        }
    }

    #[tokio::test]
    async fn test_workout_routes_postgis_support() {
        let pool = create_test_pool().await;
        // Test that workout_routes table has proper PostGIS geometry support
        let query = r#"
            SELECT column_name, data_type, udt_name
            FROM information_schema.columns
            WHERE table_name = 'workout_routes' 
            AND column_name = 'location'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        assert_eq!(rows.len(), 1, "workout_routes missing location column");
        
        let row = &rows[0];
        let data_type: String = row.get("data_type");
        assert!(data_type.contains("USER-DEFINED") || data_type.contains("geometry"), 
               "location column is not PostGIS geometry type");
    }

    #[tokio::test]
    async fn test_constraints_and_checks() {
        let pool = create_test_pool().await;
        // Test that critical constraints exist
        let query = r#"
            SELECT 
                tc.constraint_name,
                tc.table_name,
                tc.constraint_type
            FROM information_schema.table_constraints tc
            WHERE tc.table_schema = 'public'
            AND tc.constraint_type IN ('CHECK', 'UNIQUE', 'PRIMARY KEY', 'FOREIGN KEY')
            ORDER BY tc.table_name, tc.constraint_type
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        
        // Should have primary keys on all main tables
        let pk_tables: HashSet<String> = rows
            .iter()
            .filter(|row| row.get::<String, _>("constraint_type") == "PRIMARY KEY")
            .map(|row| row.get::<String, _>("table_name"))
            .collect();
        
        let required_pk_tables = vec![
            "users", "api_keys", "workout_routes", "processing_status",
            "rate_limit_tracking", "data_quality_metrics", "user_preferences"
        ];
        
        for table in required_pk_tables {
            assert!(pk_tables.contains(table), "Table '{}' missing primary key", table);
        }

        // Should have check constraints for data validation
        let check_constraints: Vec<String> = rows
            .iter()
            .filter(|row| row.get::<String, _>("constraint_type") == "CHECK")
            .map(|row| row.get::<String, _>("constraint_name"))
            .collect();
        
        assert!(!check_constraints.is_empty(), "No check constraints found");
    }

    #[tokio::test]
    async fn test_triggers_exist() {
        let pool = create_test_pool().await;
        let query = r#"
            SELECT trigger_name, event_object_table
            FROM information_schema.triggers
            WHERE trigger_schema = 'public'
        "#;
        
        let rows = sqlx::query(query).fetch_all(&pool).await.unwrap();
        
        // Should have updated_at triggers
        let trigger_tables: HashSet<String> = rows
            .iter()
            .map(|row| row.get::<String, _>("event_object_table"))
            .collect();
        
        assert!(trigger_tables.contains("users"), "users table missing updated_at trigger");
    }

    #[tokio::test]
    async fn test_partition_creation_function() {
        let pool = create_test_pool().await;
        // Test the partition creation function works
        let result = sqlx::query("SELECT create_monthly_partitions('raw_ingestions_partitioned', 'received_at', 0, 1)")
            .execute(&pool)
            .await;
        
        assert!(result.is_ok(), "create_monthly_partitions function failed");

        // Check that partitions were created
        let query = r#"
            SELECT COUNT(*) as partition_count
            FROM pg_tables 
            WHERE tablename LIKE 'raw_ingestions_partitioned_%'
            AND schemaname = 'public'
        "#;
        
        let row = sqlx::query(query).fetch_one(&pool).await.unwrap();
        let count: i64 = row.get("partition_count");
        assert!(count > 0, "No partitions created by create_monthly_partitions function");
    }

    #[tokio::test]
    async fn test_data_validation_constraints() {
        let pool = create_test_pool().await;
        // Test that data validation constraints work properly
        
        // Test heart rate constraints
        let invalid_heart_rate = sqlx::query(r#"
            INSERT INTO heart_rate_metrics_partitioned 
            (user_id, recorded_at, avg_bpm) 
            VALUES ($1, $2, $3)
        "#)
        .bind(uuid::Uuid::new_v4())
        .bind(Utc::now())
        .bind(400i16) // Invalid BPM > 300
        .execute(&pool)
        .await;
        
        assert!(invalid_heart_rate.is_err(), "Heart rate constraint not working - should reject BPM > 300");

        // Test blood pressure constraints  
        let invalid_bp = sqlx::query(r#"
            INSERT INTO blood_pressure_metrics_partitioned 
            (user_id, recorded_at, systolic, diastolic) 
            VALUES ($1, $2, $3, $4)
        "#)
        .bind(uuid::Uuid::new_v4())
        .bind(Utc::now())
        .bind(400i16) // Invalid systolic > 300
        .bind(80i16)
        .execute(&pool)
        .await;
        
        assert!(invalid_bp.is_err(), "Blood pressure constraint not working - should reject systolic > 300");
    }

    #[tokio::test]
    async fn test_performance_under_load() {
        let pool = create_test_pool().await;
        use std::time::Instant;
        
        // Insert test data and measure query performance
        let user_id = uuid::Uuid::new_v4();
        let start_time = Utc::now();
        
        // Insert 1000 heart rate records
        for i in 0..1000 {
            let recorded_at = start_time + chrono::Duration::minutes(i);
            sqlx::query(r#"
                INSERT INTO heart_rate_metrics_partitioned 
                (user_id, recorded_at, avg_bpm) 
                VALUES ($1, $2, $3)
            "#)
            .bind(user_id)
            .bind(recorded_at)
            .bind(70i16)
            .execute(&pool)
            .await
            .expect("Failed to insert test heart rate data");
        }

        // Test query performance (should be < 100ms per ARCHITECTURE.md)
        let query_start = Instant::now();
        let results = sqlx::query(r#"
            SELECT COUNT(*) as count
            FROM heart_rate_metrics_partitioned 
            WHERE user_id = $1 
            AND recorded_at >= $2
        "#)
        .bind(user_id)
        .bind(start_time)
        .fetch_all(&pool)
        .await
        .expect("Performance test query failed");
        
        let query_duration = query_start.elapsed();
        assert!(query_duration.as_millis() < 100, 
               "Query took {}ms, should be < 100ms per requirements", 
               query_duration.as_millis());
        
        assert_eq!(results.len(), 1);
        let count: i64 = results[0].get("count");
        assert_eq!(count, 1000, "Query returned incorrect count");

        // Cleanup test data
        sqlx::query("DELETE FROM heart_rate_metrics_partitioned WHERE user_id = $1")
            .bind(user_id)
            .execute(&pool)
            .await
            .expect("Failed to cleanup test data");
    }

    #[tokio::test]
    async fn test_index_usage() {
        let pool = create_test_pool().await;
        // Test that queries use appropriate indexes
        let user_id = uuid::Uuid::new_v4();
        
        let explain_result = sqlx::query(r#"
            EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON)
            SELECT * FROM heart_rate_metrics_partitioned 
            WHERE user_id = $1 
            AND recorded_at >= NOW() - INTERVAL '7 days'
        "#)
        .bind(user_id)
        .fetch_one(&pool)
        .await
        .expect("EXPLAIN query failed");
        
        let plan: serde_json::Value = explain_result.get("QUERY PLAN");
        let plan_str = plan.to_string();
        
        // Should use index scan, not sequential scan for optimal performance
        assert!(!plan_str.contains("Seq Scan"), 
               "Query using sequential scan instead of index: {}", plan_str);
    }
}