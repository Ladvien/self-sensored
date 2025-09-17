use actix_web::{test, web, App};
use serde_json::Value;
use std::env;

use self_sensored::{
    db::database::create_connection_pool,
    handlers::health::{health_check, liveness_probe},
};

async fn get_test_pool() -> sqlx::PgPool {
    dotenv::dotenv().ok();
    let database_url = env::var("TEST_DATABASE_URL")
        .or_else(|_| env::var("DATABASE_URL"))
        .expect("TEST_DATABASE_URL or DATABASE_URL must be set");
    create_connection_pool(&database_url)
        .await
        .expect("Failed to create test database pool")
}

#[tokio::test]
async fn test_basic_health_check_works() {
    let app = test::init_service(
        App::new().route("/health", web::get().to(health_check)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health").to_request();
    let resp = test::call_service(&app, req).await;

    assert!(resp.status().is_success());

    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "healthy");
    assert!(body["check_id"].is_number());
}

#[tokio::test]
async fn test_liveness_probe_works() {
    let app = test::init_service(
        App::new().route("/health/live", web::get().to(liveness_probe)),
    )
    .await;

    let req = test::TestRequest::get().uri("/health/live").to_request();
    let resp = test::call_service(&app, req).await;

    assert_eq!(resp.status(), 200);
    let body: Value = test::read_body_json(resp).await;
    assert_eq!(body["status"], "alive");
}