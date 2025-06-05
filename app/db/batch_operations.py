# app/db/batch_operations.py - Optimized batch operations

import hashlib
import json
from typing import Dict, Any, Optional, List, Type, Generator, Tuple
from sqlalchemy.ext.asyncio import AsyncSession
from sqlalchemy.dialects.postgresql import insert
from sqlalchemy import select, text
from uuid import uuid4
import logging
from itertools import islice
from contextlib import asynccontextmanager

from app.api.models import HealthPayload, HealthMetric, METRIC_CONFIGS, TimestampedModel
import app.db.models as db_models

logger = logging.getLogger(__name__)


class BatchProcessor:
    """Handles optimized batch operations for health data"""

    def __init__(self, db_session: AsyncSession, batch_size: int = 1000):
        self.db = db_session
        self.batch_size = batch_size
        self.metrics_processed = 0
        self.metrics_skipped = 0

    @staticmethod
    def generate_payload_hash(payload_data: dict) -> str:
        """Generate deterministic hash for payload deduplication"""
        normalized = json.dumps(payload_data, sort_keys=True, default=str)
        return hashlib.sha256(normalized.encode()).hexdigest()

    @staticmethod
    def generate_metric_hash(
        metric_name: str, metric_data: List[Dict[str, Any]]
    ) -> str:
        """Generate hash for metric data deduplication"""
        data_str = json.dumps(
            {"name": metric_name, "data": metric_data}, sort_keys=True, default=str
        )
        return hashlib.md5(data_str.encode()).hexdigest()

    def chunked_iterable(self, iterable, size: int) -> Generator[List, None, None]:
        """Split iterable into chunks of specified size"""
        it = iter(iterable)
        while chunk := list(islice(it, size)):
            yield chunk

    async def check_payload_exists(self, payload_hash: str) -> Optional[str]:
        """Check if payload already exists, return ID if found"""
        result = await self.db.execute(
            select(db_models.HealthPayload.id).where(
                db_models.HealthPayload.payload_hash == payload_hash
            )
        )
        return result.scalar_one_or_none()

    async def insert_payload_idempotent(self, payload_hash: str) -> str:
        """Insert payload with conflict resolution"""
        payload_id = uuid4()

        stmt = insert(db_models.HealthPayload).values(
            id=payload_id, payload_hash=payload_hash
        )

        # Handle race conditions with upsert
        stmt = stmt.on_conflict_do_update(
            index_elements=["payload_hash"],
            set_=dict(received_at=stmt.excluded.received_at),
        ).returning(db_models.HealthPayload.id)

        result = await self.db.execute(stmt)
        return result.scalar_one()

    async def insert_metric_batch(
        self, metrics: List[HealthMetric], payload_id: str
    ) -> Tuple[int, int]:
        """Insert metrics in optimized batches"""
        processed = 0
        skipped = 0

        # Prepare metric records for batch insert
        metric_records = []
        metric_data_map = {}  # metric_id -> parsed_data

        for metric in metrics:
            # Generate hash for deduplication
            metric_data_hash = self.generate_metric_hash(
                metric.name,
                [
                    item.model_dump() if hasattr(item, "model_dump") else item
                    for item in metric.data
                ],
            )

            metric_id = uuid4()
            metric_records.append(
                {
                    "id": metric_id,
                    "payload_id": payload_id,
                    "name": metric.name,
                    "units": metric.units,
                    "data_hash": metric_data_hash,
                }
            )

            # Store parsed data for later insertion
            metric_data_map[metric_id] = metric.data

        # Batch insert metrics with conflict resolution
        if metric_records:

            # Calculate safe batch size based on parameter limits
            effective_batch_size = (
                min(batch_size, 1000 // len(records[0].keys()))
                if records
                else batch_size
            )

            for chunk in chunked_iterable(records, effective_batch_size):
                stmt = insert(db_models.HealthMetric).values(chunk)
                stmt = stmt.on_conflict_do_nothing(
                    index_elements=["payload_id", "name", "data_hash"]
                ).returning(db_models.HealthMetric.id, db_models.HealthMetric.name)

                result = await self.db.execute(stmt)
                inserted_metrics = result.fetchall()

                # Track metrics that were actually inserted
                for metric_id, metric_name in inserted_metrics:
                    if metric_id in metric_data_map:
                        # Insert metric data
                        await self._insert_metric_data(
                            metric_data_map[metric_id], metric_id, metric_name
                        )
                        processed += 1
                    else:
                        print(f"Metric ID {metric_id} not found in data map, skipping")
                        skipped += 1

        return processed, skipped

    async def _insert_metric_data(
        self, data_entries: List[TimestampedModel], metric_id: str, metric_name: str
    ) -> None:
        """Insert metric data using appropriate specialized table"""
        if not data_entries:
            return

        config = METRIC_CONFIGS.get(metric_name.lower())

        if config is None:
            # Use generic quantity table
            await self._insert_quantity_data_batch(data_entries, metric_id)
        else:
            # Use specialized table
            await self._insert_specialized_data_batch(data_entries, metric_id, config)

    async def _insert_quantity_data_batch(
        self, data_entries: List[TimestampedModel], metric_id: str
    ) -> None:
        """Batch insert quantity data with optimized conflict resolution"""
        records = []

        for entry in data_entries:
            try:
                record = {
                    "id": uuid4(),
                    "metric_id": metric_id,
                    "date": entry.get_primary_date(),
                    "qty": entry.qty,
                    "source": getattr(entry, "source", None),
                }
                records.append(record)
            except Exception as e:
                logger.warning(f"Skipping invalid quantity entry: {e}")

        if not records:
            return

        # Use COPY for large batches, INSERT for smaller ones
        if len(records) > 5000:
            await self._bulk_copy_insert(records, "quantity_timestamp")
        else:
            await self._batch_upsert(
                records, db_models.QuantityTimestamp, ["metric_id", "date", "source"]
            )

    async def _insert_specialized_data_batch(
        self, data_entries: List[TimestampedModel], metric_id: str, config
    ) -> None:
        """Batch insert specialized metric data"""
        records = []

        for entry in data_entries:
            try:
                record = entry.model_dump(exclude={"id"}, exclude_unset=True)

                # Handle field mapping inconsistencies
                if "timestamp" in record:
                    record["date"] = record.pop("timestamp")

                record.update(
                    {
                        "id": uuid4(),
                        "metric_id": metric_id,
                    }
                )
                records.append(record)

            except Exception as e:
                logger.warning(f"Skipping invalid {config.table_name} entry: {e}")

        if not records:
            return

        # Get the SQLAlchemy model
        model_cls = db_models.MODEL_REGISTRY.get(config.table_name)
        if model_cls:
            await self._batch_upsert(records, model_cls, config.conflict_fields)

    async def _batch_upsert(
        self, records: List[Dict], model_cls: Type, conflict_fields: List[str]
    ) -> None:
        """Perform batch upsert with conflict resolution"""
        for chunk in self.chunked_iterable(records, self.batch_size):
            stmt = insert(model_cls).values(chunk)

            # Build update dict for conflict resolution
            update_dict = {
                field: getattr(stmt.excluded, field)
                for field in chunk[0].keys()
                if field not in conflict_fields and field != "id"
            }

            if update_dict:
                stmt = stmt.on_conflict_do_update(
                    index_elements=conflict_fields, set_=update_dict
                )
            else:
                stmt = stmt.on_conflict_do_nothing(index_elements=conflict_fields)

            await self.db.execute(stmt)

    async def _bulk_copy_insert(self, records: List[Dict], table_name: str) -> None:
        """Use PostgreSQL COPY for very large batches"""
        # This would require implementing a CSV conversion and COPY command
        # For now, fall back to regular batch insert
        logger.info(
            f"Large batch detected ({len(records)} records), using batch insert"
        )

        model_cls = db_models.MODEL_REGISTRY.get(table_name)
        if model_cls:
            await self._batch_upsert(
                records, model_cls, ["metric_id", "date", "source"]
            )

    async def insert_workouts_batch(self, workouts: List, payload_id: str) -> None:
        """Batch insert workout data"""
        if not workouts:
            return

        workout_records = []
        workout_values_batch = []
        workout_points_batch = []
        route_points_batch = []

        for workout in workouts:
            workout_id = uuid4()

            # Main workout record
            workout_records.append(
                {
                    "id": workout_id,
                    "payload_id": payload_id,
                    "name": workout.name,
                    "start": workout.start,
                    "end": workout.end,
                    "elevation": (
                        workout.elevation.model_dump() if workout.elevation else None
                    ),
                }
            )

            # Workout values
            for attr_name, attr_value in [
                ("total_energy", workout.total_energy),
                ("active_energy", workout.active_energy),
                ("max_heart_rate", workout.max_heart_rate),
                ("avg_heart_rate", workout.avg_heart_rate),
                ("step_count", workout.step_count),
                ("step_cadence", workout.step_cadence),
                ("total_swimming_stroke_count", workout.total_swimming_stroke_count),
                ("swim_cadence", workout.swim_cadence),
                ("distance", workout.distance),
                ("speed", workout.speed),
                ("flights_climbed", workout.flights_climbed),
                ("intensity", getattr(workout, "intensity", None)),
                ("temperature", workout.temperature),
                ("humidity", workout.humidity),
            ]:
                if attr_value:
                    workout_values_batch.append(
                        {
                            "id": uuid4(),
                            "workout_id": workout_id,
                            "name": attr_name,
                            "qty": attr_value.qty,
                            "units": attr_value.units,
                        }
                    )

            # Workout points (heart rate data)
            for stream_name, points in [
                ("heart_rate", workout.heart_rate),
                ("heart_rate_recovery", workout.heart_rate_recovery),
            ]:
                if points:
                    for point in points:
                        workout_points_batch.append(
                            {
                                "id": uuid4(),
                                "workout_id": workout_id,
                                "stream": stream_name,
                                "date": point.date,
                                "qty": point.qty,
                                "units": point.units,
                            }
                        )

            # Route points
            if workout.route:
                for point in workout.route:
                    route_points_batch.append(
                        {
                            "id": uuid4(),
                            "workout_id": workout_id,
                            "lat": point.lat,
                            "lon": point.lon,
                            "altitude": point.altitude,
                            "timestamp": point.timestamp,
                        }
                    )

        # Batch insert all workout data
        await self._batch_upsert(
            workout_records, db_models.Workout, ["payload_id", "name", "start"]
        )

        if workout_values_batch:
            await self._batch_upsert(
                workout_values_batch, db_models.WorkoutValue, ["workout_id", "name"]
            )

        if workout_points_batch:
            await self._batch_upsert(
                workout_points_batch,
                db_models.WorkoutPoint,
                ["workout_id", "stream", "date"],
            )

        if route_points_batch:
            await self._batch_upsert(
                route_points_batch,
                db_models.WorkoutRoutePoint,
                ["workout_id", "timestamp"],
            )


@asynccontextmanager
async def batch_transaction(db_session: AsyncSession):
    """Context manager for batch operations with transaction handling"""
    try:
        yield
        await db_session.commit()
    except Exception as e:
        await db_session.rollback()
        logger.error(f"Batch transaction failed: {e}")
        raise


async def insert_health_data_optimized(
    payload: HealthPayload, db: AsyncSession, raw_payload: dict = None
) -> Dict[str, Any]:
    """
    Optimized health data insertion with improved batch processing

    Returns:
        dict: Status information with processing metrics
    """
    # Generate payload hash for deduplication
    payload_dict = raw_payload or payload.model_dump()
    payload_hash = BatchProcessor.generate_payload_hash(payload_dict)

    processor = BatchProcessor(db)

    # Check for existing payload
    existing_payload_id = await processor.check_payload_exists(payload_hash)
    if existing_payload_id:
        logger.info(f"Payload {payload_hash[:8]}... already exists")
        return {
            "status": "duplicate",
            "message": "Payload already processed",
            "payload_id": str(existing_payload_id),
            "metrics_processed": 0,
            "metrics_skipped": len(payload.metrics),
        }

    async with batch_transaction(db):
        # Insert payload
        payload_id = await processor.insert_payload_idempotent(payload_hash)

        # Batch process metrics
        metrics_processed, metrics_skipped = await processor.insert_metric_batch(
            payload.metrics, payload_id
        )

        # Batch process workouts
        if hasattr(payload, "workouts") and payload.workouts:
            await processor.insert_workouts_batch(payload.workouts, payload_id)

        status = "success" if metrics_skipped == 0 else "partial_success"

        logger.info(
            f"Processed payload {payload_hash[:8]}...: "
            f"{metrics_processed} metrics added, {metrics_skipped} skipped"
        )

        return {
            "status": status,
            "message": f"Data processed: {metrics_processed} new metrics, {metrics_skipped} duplicates",
            "payload_id": str(payload_id),
            "metrics_processed": metrics_processed,
            "metrics_skipped": metrics_skipped,
        }
