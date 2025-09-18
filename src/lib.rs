pub mod config;
pub mod db;
pub mod handlers;
pub mod middleware;
pub mod models;
pub mod services;

#[cfg(test)]
mod validation {
    include!("../tests/validation/parameter_validation_vs_processing_mismatch_test.rs");
}
