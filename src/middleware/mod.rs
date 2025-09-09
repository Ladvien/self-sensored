// Middleware modules for authentication, rate limiting, metrics, and performance
pub mod auth;
pub mod compression;
pub mod logging;
pub mod metrics;
pub mod request_logger;
// pub mod rate_limit; // Rate limiting disabled for now

pub use auth::*;
pub use compression::*;
pub use logging::*;
pub use metrics::*;
pub use request_logger::*;
