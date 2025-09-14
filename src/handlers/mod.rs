pub mod admin;
pub mod auth;
pub mod background;
pub mod body_measurements_handler;
pub mod environmental_handler;
pub mod export;
pub mod health;
pub mod ingest;
pub mod ingest_async_simple;
pub mod metabolic_handler;
pub mod mindfulness_handler;
pub mod nutrition_handler;
pub mod optimized_ingest;
pub mod query;
pub mod reproductive_health_handler;
pub mod respiratory_handler;
pub mod symptoms_handler;
pub mod temperature_handler;

// New modular architecture components
pub mod background_coordinator;
pub mod data_loader;
pub mod payload_processor;
pub mod streaming_processor;
pub mod timeout_manager;
