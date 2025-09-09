// Health service for system status checks

use sqlx::PgPool;

pub struct HealthService;

impl HealthService {
    pub async fn check_database_connection(pool: &PgPool) -> bool {
        (sqlx::query("SELECT 1").fetch_one(pool).await).is_ok()
    }
}
