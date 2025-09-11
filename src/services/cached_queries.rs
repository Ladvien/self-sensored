use actix_web::Result;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::time::Duration;
use tracing::{info, instrument};
use uuid::Uuid;

use crate::handlers::query::{
    get_activity_summary, get_blood_pressure_summary, get_heart_rate_summary, get_sleep_summary,
    get_workout_summary, HealthSummary, QueryParams, QueryResponse,
};
use crate::models::db::*;
use crate::services::auth::AuthContext;
use crate::services::cache::{generate_query_hash, CacheKey, CacheService};

/// Cached query service that wraps database queries with Redis caching
#[derive(Clone)]
pub struct CachedQueryService {
    pool: PgPool,
    cache: CacheService,
    cache_prefix: String,
    cache_enabled: bool,
}

impl CachedQueryService {
    pub fn new(pool: PgPool, cache: CacheService, cache_prefix: String) -> Self {
        let cache_enabled = cache.is_enabled();
        Self {
            pool,
            cache,
            cache_prefix,
            cache_enabled,
        }
    }

    /// Get heart rate data with caching
    #[instrument(skip(self, auth))]
    pub async fn get_heart_rate_data_cached(
        &self,
        auth: AuthContext,
        params: &QueryParams,
    ) -> Result<QueryResponse<HeartRateRecord>> {
        let query_hash = generate_query_hash(params);
        let cache_key = CacheKey::HeartRateQuery {
            user_id: auth.user.id,
            hash: query_hash,
        };

        // Try cache first
        if self.cache_enabled {
            if let Some(cached_result) = self
                .cache
                .get::<QueryResponse<HeartRateRecord>>(&cache_key, &self.cache_prefix)
                .await
            {
                info!(
                    user_id = %auth.user.id,
                    "Heart rate data served from cache"
                );
                return Ok(cached_result);
            }
        }

        // Cache miss - query database
        let result = self
            .query_heart_rate_from_db(auth.user.id, params)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Cache the result
        if self.cache_enabled {
            let cache_ttl = Some(Duration::from_secs(600)); // 10 minutes
            self.cache
                .set(&cache_key, &self.cache_prefix, result.clone(), cache_ttl)
                .await;
        }

        Ok(result)
    }

    /// Get comprehensive health summary with aggressive caching
    #[instrument(skip(self, auth))]
    pub async fn get_health_summary_cached(
        &self,
        auth: AuthContext,
        params: &QueryParams,
    ) -> Result<HealthSummary> {
        let start_date = params
            .start_date
            .unwrap_or_else(|| Utc::now() - chrono::Duration::days(30));
        let end_date = params.end_date.unwrap_or_else(|| Utc::now());

        let date_range = format!(
            "{}_{}",
            start_date.format("%Y%m%d"),
            end_date.format("%Y%m%d")
        );

        let cache_key = CacheKey::HealthSummary {
            user_id: auth.user.id,
            date_range,
        };

        // Try cache first - summaries are expensive to compute
        if self.cache_enabled {
            if let Some(cached_summary) = self
                .cache
                .get::<HealthSummary>(&cache_key, &self.cache_prefix)
                .await
            {
                info!(
                    user_id = %auth.user.id,
                    "Health summary served from cache"
                );
                return Ok(cached_summary);
            }
        }

        // Cache miss - compute summary
        let summary = self
            .compute_health_summary(auth.user.id, start_date, end_date)
            .await
            .map_err(|e| actix_web::error::ErrorInternalServerError(e))?;

        // Cache with longer TTL for summaries
        if self.cache_enabled {
            let cache_ttl = Some(Duration::from_secs(1800)); // 30 minutes
            self.cache
                .set(&cache_key, &self.cache_prefix, summary.clone(), cache_ttl)
                .await;
        }

        Ok(summary)
    }

    /// Invalidate cache when new data is ingested
    #[instrument(skip(self))]
    pub async fn invalidate_user_cache(&self, user_id: Uuid) {
        if self.cache_enabled {
            self.cache
                .invalidate_user_cache(user_id, &self.cache_prefix)
                .await;
            info!(
                user_id = %user_id,
                "Cache invalidated after data ingestion"
            );
        }
    }

    /// Get cache performance statistics
    pub async fn get_cache_stats(&self) -> crate::services::cache::CacheStats {
        self.cache.get_stats().await
    }

    // Private helper methods

    async fn query_heart_rate_from_db(
        &self,
        user_id: Uuid,
        params: &QueryParams,
    ) -> Result<QueryResponse<HeartRateRecord>, sqlx::Error> {
        let page = params.page.unwrap_or(1).max(1);
        let limit = params.limit.unwrap_or(100).min(1000);
        let offset = (page - 1) * limit;
        let sort_order = match params.sort.as_deref() {
            Some("asc") => "ASC",
            _ => "DESC",
        };

        // Build dynamic query with date filtering
        let mut query = format!(
            r#"
            SELECT user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device, metadata, created_at
            FROM heart_rate_metrics 
            WHERE user_id = $1
            "#
        );

        let mut param_count = 2;
        if params.start_date.is_some() {
            query.push_str(&format!(" AND recorded_at >= ${param_count}"));
            param_count += 1;
        }
        if params.end_date.is_some() {
            query.push_str(&format!(" AND recorded_at <= ${param_count}"));
            param_count += 1;
        }

        query.push_str(&format!(
            " ORDER BY recorded_at {sort_order} LIMIT ${param_count} OFFSET ${}",
            param_count + 1
        ));

        let mut db_query = sqlx::query_as::<_, HeartRateRecord>(&query).bind(user_id);

        if let Some(start_date) = params.start_date {
            db_query = db_query.bind(start_date);
        }
        if let Some(end_date) = params.end_date {
            db_query = db_query.bind(end_date);
        }

        db_query = db_query.bind(limit as i64).bind(offset as i64);

        let records = db_query.fetch_all(&self.pool).await?;
        let total_count = self
            .get_heart_rate_count(user_id, params)
            .await
            .unwrap_or(0);

        let pagination = crate::handlers::query::PaginationInfo {
            page,
            limit,
            has_next: (offset + limit) < total_count as u32,
            has_prev: page > 1,
        };

        Ok(QueryResponse {
            data: records,
            pagination,
            total_count,
        })
    }

    async fn get_heart_rate_count(
        &self,
        user_id: Uuid,
        params: &QueryParams,
    ) -> Result<i64, sqlx::Error> {
        let mut query = "SELECT COUNT(*) FROM heart_rate_metrics WHERE user_id = $1".to_string();
        let mut param_count = 2;

        if params.start_date.is_some() {
            query.push_str(&format!(" AND recorded_at >= ${param_count}"));
            param_count += 1;
        }
        if params.end_date.is_some() {
            query.push_str(&format!(" AND recorded_at <= ${param_count}"));
        }

        let mut db_query = sqlx::query_scalar(&query).bind(user_id);
        if let Some(start_date) = params.start_date {
            db_query = db_query.bind(start_date);
        }
        if let Some(end_date) = params.end_date {
            db_query = db_query.bind(end_date);
        }

        db_query.fetch_one(&self.pool).await
    }

    async fn compute_health_summary(
        &self,
        user_id: Uuid,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<HealthSummary, sqlx::Error> {
        let date_range = crate::handlers::query::DateRange {
            start_date,
            end_date,
        };

        // Run all summary queries in parallel for better performance
        let (heart_rate_result, bp_result, sleep_result, activity_result, workout_result) = tokio::join!(
            get_heart_rate_summary(&self.pool, user_id, start_date, end_date),
            get_blood_pressure_summary(&self.pool, user_id, start_date, end_date),
            get_sleep_summary(&self.pool, user_id, start_date, end_date),
            get_activity_summary(&self.pool, user_id, start_date, end_date),
            get_workout_summary(&self.pool, user_id, start_date, end_date)
        );

        Ok(HealthSummary {
            user_id,
            date_range,
            heart_rate: heart_rate_result.ok(),
            blood_pressure: bp_result.ok(),
            sleep: sleep_result.ok(),
            activity: activity_result.ok(),
            workouts: workout_result.ok(),
        })
    }
}
