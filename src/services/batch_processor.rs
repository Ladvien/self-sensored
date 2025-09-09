use sqlx::{PgPool, Postgres, QueryBuilder};
use tracing::{info, instrument};
use uuid::Uuid;

use crate::models::{HealthMetric, IngestPayload, ProcessingError};

/// Batch processing service for health data
pub struct BatchProcessor {
    pool: PgPool,
}

/// Result of batch processing operation
#[derive(Debug)]
pub struct BatchProcessingResult {
    pub processed_count: usize,
    pub failed_count: usize,
    pub errors: Vec<ProcessingError>,
}

impl BatchProcessor {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Process a batch of health data with comprehensive error handling
    #[instrument(skip(self, payload))]
    pub async fn process_batch(
        &self,
        user_id: Uuid,
        payload: IngestPayload,
    ) -> BatchProcessingResult {
        let mut result = BatchProcessingResult {
            processed_count: 0,
            failed_count: 0,
            errors: Vec::new(),
        };

        // Group metrics by type for batch processing
        let mut grouped = self.group_metrics_by_type(payload.data.metrics);

        // Process heart rate metrics
        if !grouped.heart_rates.is_empty() {
            let heart_rates = std::mem::take(&mut grouped.heart_rates);
            match self.insert_heart_rates(user_id, heart_rates).await {
                Ok(count) => result.processed_count += count,
                Err(e) => {
                    result.failed_count += 1;
                    result.errors.push(ProcessingError {
                        metric_type: "HeartRate".to_string(),
                        error_message: e.to_string(),
                        index: None,
                    });
                }
            }
        }

        // Process blood pressure metrics
        if !grouped.blood_pressures.is_empty() {
            let blood_pressures = std::mem::take(&mut grouped.blood_pressures);
            match self.insert_blood_pressures(user_id, blood_pressures).await {
                Ok(count) => result.processed_count += count,
                Err(e) => {
                    result.failed_count += 1;
                    result.errors.push(ProcessingError {
                        metric_type: "BloodPressure".to_string(),
                        error_message: e.to_string(),
                        index: None,
                    });
                }
            }
        }

        // Process sleep metrics
        if !grouped.sleep_metrics.is_empty() {
            let sleep_metrics = std::mem::take(&mut grouped.sleep_metrics);
            match self.insert_sleep_metrics(user_id, sleep_metrics).await {
                Ok(count) => result.processed_count += count,
                Err(e) => {
                    result.failed_count += 1;
                    result.errors.push(ProcessingError {
                        metric_type: "Sleep".to_string(),
                        error_message: e.to_string(),
                        index: None,
                    });
                }
            }
        }

        // Process activity metrics
        if !grouped.activities.is_empty() {
            let activities = std::mem::take(&mut grouped.activities);
            match self.insert_activities(user_id, activities).await {
                Ok(count) => result.processed_count += count,
                Err(e) => {
                    result.failed_count += 1;
                    result.errors.push(ProcessingError {
                        metric_type: "Activity".to_string(),
                        error_message: e.to_string(),
                        index: None,
                    });
                }
            }
        }

        // Process workouts from both metrics and dedicated workout list
        let mut all_workouts = grouped.workouts;
        all_workouts.extend(payload.data.workouts);

        if !all_workouts.is_empty() {
            match self.insert_workouts(user_id, all_workouts).await {
                Ok(count) => result.processed_count += count,
                Err(e) => {
                    result.failed_count += 1;
                    result.errors.push(ProcessingError {
                        metric_type: "Workout".to_string(),
                        error_message: e.to_string(),
                        index: None,
                    });
                }
            }
        }

        info!(
            user_id = %user_id,
            processed = result.processed_count,
            failed = result.failed_count,
            errors = result.errors.len(),
            "Batch processing completed"
        );

        result
    }

    /// Group metrics by type for efficient batch processing
    fn group_metrics_by_type(&self, metrics: Vec<HealthMetric>) -> GroupedMetrics {
        let mut grouped = GroupedMetrics::default();

        for metric in metrics {
            match metric {
                HealthMetric::HeartRate(hr) => grouped.heart_rates.push(hr),
                HealthMetric::BloodPressure(bp) => grouped.blood_pressures.push(bp),
                HealthMetric::Sleep(sleep) => grouped.sleep_metrics.push(sleep),
                HealthMetric::Activity(activity) => grouped.activities.push(activity),
                HealthMetric::Workout(workout) => grouped.workouts.push(workout),
            }
        }

        grouped
    }

    /// Batch insert heart rate metrics with ON CONFLICT handling
    async fn insert_heart_rates(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::HeartRateMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO heart_rate_metrics (user_id, recorded_at, heart_rate, resting_heart_rate, context, source_device) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            // Use avg_bpm as the primary heart_rate value, or max_bpm if no avg
            let heart_rate = metric.avg_bpm.or(metric.max_bpm).unwrap_or(0);
            // Use min_bpm as resting heart rate if it's significantly different
            let resting_heart_rate = if metric.min_bpm.is_some() && metric.min_bpm != metric.avg_bpm
            {
                metric.min_bpm
            } else {
                None
            };

            b.push_bind(user_id)
                .push_bind(metric.recorded_at)
                .push_bind(heart_rate)
                .push_bind(resting_heart_rate)
                .push_bind(&metric.context)
                .push_bind(&metric.source);
        });

        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

        let result = query_builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Batch insert blood pressure metrics
    async fn insert_blood_pressures(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::BloodPressureMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO blood_pressure_metrics (user_id, recorded_at, systolic, diastolic, pulse, source) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            b.push_bind(user_id)
                .push_bind(metric.recorded_at)
                .push_bind(metric.systolic)
                .push_bind(metric.diastolic)
                .push_bind(metric.pulse)
                .push_bind(&metric.source);
        });

        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

        let result = query_builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Batch insert sleep metrics
    async fn insert_sleep_metrics(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::SleepMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO sleep_metrics (user_id, recorded_at, sleep_start, sleep_end, total_sleep_minutes, deep_sleep_minutes, rem_sleep_minutes, awake_minutes, efficiency_percentage, source) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            b.push_bind(user_id)
                .push_bind(metric.recorded_at)
                .push_bind(metric.sleep_start)
                .push_bind(metric.sleep_end)
                .push_bind(metric.total_sleep_minutes)
                .push_bind(metric.deep_sleep_minutes)
                .push_bind(metric.rem_sleep_minutes)
                .push_bind(metric.awake_minutes)
                .push_bind(metric.efficiency_percentage)
                .push_bind(&metric.source);
        });

        query_builder.push(" ON CONFLICT (user_id, recorded_at) DO NOTHING");

        let result = query_builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Batch insert activity metrics (daily aggregates)
    async fn insert_activities(
        &self,
        user_id: Uuid,
        metrics: Vec<crate::models::ActivityMetric>,
    ) -> Result<usize, sqlx::Error> {
        if metrics.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO activity_metrics (user_id, date, steps, distance_meters, calories_burned, active_minutes, flights_climbed, source) "
        );

        query_builder.push_values(metrics.iter(), |mut b, metric| {
            b.push_bind(user_id)
                .push_bind(metric.date)
                .push_bind(metric.steps)
                .push_bind(metric.distance_meters)
                .push_bind(metric.calories_burned)
                .push_bind(metric.active_minutes)
                .push_bind(metric.flights_climbed)
                .push_bind(&metric.source);
        });

        // Activities are unique per user/date, use ON CONFLICT to update
        query_builder.push(
            " ON CONFLICT (user_id, date) DO UPDATE SET
              steps = COALESCE(EXCLUDED.steps, activity_metrics.steps),
              distance_meters = COALESCE(EXCLUDED.distance_meters, activity_metrics.distance_meters),
              calories_burned = COALESCE(EXCLUDED.calories_burned, activity_metrics.calories_burned),
              active_minutes = COALESCE(EXCLUDED.active_minutes, activity_metrics.active_minutes),
              flights_climbed = COALESCE(EXCLUDED.flights_climbed, activity_metrics.flights_climbed),
              updated_at = NOW()"
        );

        let result = query_builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as usize)
    }

    /// Batch insert workout records
    async fn insert_workouts(
        &self,
        user_id: Uuid,
        workouts: Vec<crate::models::WorkoutData>,
    ) -> Result<usize, sqlx::Error> {
        if workouts.is_empty() {
            return Ok(0);
        }

        let mut query_builder: QueryBuilder<Postgres> = QueryBuilder::new(
            "INSERT INTO workouts (id, user_id, workout_type, start_time, end_time, total_energy_kcal, distance_meters, avg_heart_rate, max_heart_rate, source) "
        );

        query_builder.push_values(workouts.iter(), |mut b, workout| {
            b.push_bind(Uuid::new_v4()) // Generate new ID for each workout
                .push_bind(user_id)
                .push_bind(&workout.workout_type)
                .push_bind(workout.start_time)
                .push_bind(workout.end_time)
                .push_bind(workout.total_energy_kcal)
                .push_bind(workout.distance_meters)
                .push_bind(workout.avg_heart_rate)
                .push_bind(workout.max_heart_rate)
                .push_bind(&workout.source);
        });

        // Workouts can have duplicates based on start_time
        query_builder.push(" ON CONFLICT (user_id, start_time) DO NOTHING");

        let result = query_builder.build().execute(&self.pool).await?;
        Ok(result.rows_affected() as usize)
    }
}

/// Helper struct for grouping metrics by type
#[derive(Default)]
struct GroupedMetrics {
    heart_rates: Vec<crate::models::HeartRateMetric>,
    blood_pressures: Vec<crate::models::BloodPressureMetric>,
    sleep_metrics: Vec<crate::models::SleepMetric>,
    activities: Vec<crate::models::ActivityMetric>,
    workouts: Vec<crate::models::WorkoutData>,
}
